use crate::bokio::{Bokio, CreateJournal, CreateJournalAccount, JournalEntry, BOKIO_API_URL};
use crate::eskassa::{DateRange, DinKassa, SIEReportListItem, ZReportListItem};
use crate::utils::{format_local_date, money};
use chrono::naive::NaiveDate;
use chrono::Days;
use rust_decimal::{dec, Decimal};
use std::collections::HashMap;
use std::io::Write;
use std::iter::{once, repeat_n};
use std::str::FromStr;
use tabled::{builder::Builder, settings::Alignment, settings::Padding, settings::Style};
use ureq::Error;
use utils::{read_password_trim, read_prompt_trim, to_date};

mod bokio;
mod eskassa;
mod utils;

struct Cli {
    dinkassa_username: String,
    dinkassa_password: String,
    bokio_api_url: String,
    bokio_api_token: String,
    bokio_company_id: String,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    save_files: bool,
}

fn check_arg(name: &str, arg: &str, iter: &mut impl Iterator<Item = String>) -> Option<String> {
    let prefix = "--".to_string() + name;
    if arg == prefix {
        let val = iter.next();
        return Some(val.expect(&format!("{} expected value", prefix)));
    }

    let prefix = prefix + "=";
    if let Some(val) = arg.strip_prefix(&prefix) {
        let val = Some(val)
            .filter(|s| !s.is_empty())
            .expect(&format!("{} expected value", prefix));
        return Some(val.to_string());
    }

    None
}

struct RapportImport {
    sie: SIEReportListItem,
    report: ZReportListItem,
    verifikat: Option<JournalEntry>,
}

fn hamta_rapporter(
    args: &Cli,
    dinkassa: &DinKassa,
    bokio: &Bokio,
) -> Result<(Vec<RapportImport>, DateRange), Error> {
    let interval = DateRange::new(&args.start_date, &args.end_date);
    let bokio_start_date = interval.start_date.checked_sub_days(Days::new(14)).unwrap();
    /*
    if bokio_start_date.year() < interval.start_date.year() {
        bokio_start_date = NaiveDate::from_ymd_opt(interval.start_date.year(), 1, 1).unwrap();
    }
   */
    let journal = bokio.list_journal(Some(bokio_start_date), Some(interval.end_date))?;
    let mut importer: Vec<RapportImport> = Vec::new();
    let sie_listing = dinkassa.list_sie_reports(&interval)?;
    let report_listing = dinkassa.list_zreports(&interval)?;
    for sie in sie_listing.zreports {
        let title = sie.verifikatnamn().to_lowercase();
        let nr = sie.number().expect(&format!("Kunde inte tolka rapport {}", sie.zreport));
        let report = report_listing.items.iter()
            .find(|e| e.number == nr)
            .expect(&format!("Z-Rapport {} hittades inte", nr))
            .clone();
        let verifikat = journal
            .iter()
            .find(|e| e.title.to_lowercase() == title && e.reversed_by_journal_entry_id.is_none())
            .cloned();
        importer.push(RapportImport {
            sie,
            report,
            verifikat,
        })
    }

    Ok((importer, interval))
}

fn rakna_importerade_rapporter(importer: &Vec<RapportImport>) -> usize {
    importer.iter().filter(|e| e.verifikat.is_some()).count()
}

fn lista_rapporter(importer: &Vec<RapportImport>) {
    let mut builder = Builder::default();
    let mut account_names: HashMap<String, String> = HashMap::new();
    for e in importer {
        for acc in e.sie.accounts.iter() {
            if !account_names.contains_key(&acc.number) && (acc.number.starts_with("1")) {
                account_names.insert(acc.number.clone(), acc.description.to_uppercase());
            }
        }
    }

    let mut accounts: Vec<String> = account_names.keys().map(|k| k.to_string()).collect();
    let mut account_totals: Vec<Decimal> = repeat_n(Decimal::ZERO, accounts.len()).collect();
    accounts.sort();
    let fixed_columns = ["VERIFIKAT", "Z-RAPPORT", "DATUM"];
    builder.push_record(fixed_columns.iter().map(|a| a.to_string())
        .chain(accounts.iter().map(|a| account_names[a].clone()))
        .chain(once("TOTAL".to_string())));
    for e in importer {
        let rapport = &e.sie;
        let verifikat = &e.verifikat;
        let vernr = verifikat
            .clone()
            .map_or("".to_string(), |j| j.journal_entry_number + " ✓");
        let datum = rapport.datum();
        let number = e.report.number.to_string();
        let mut values: Vec<String> = [vernr, number, datum].to_vec();
        let mut total = dec!(0);
        for (i, acc) in accounts.iter().enumerate() {
            let amount = rapport.konto(&acc).unwrap_or(Decimal::ZERO);
            values.push(money(amount));
            account_totals[i] += amount;
            if amount.is_sign_positive() {
                total += amount;
            }
        }
        values.push(money(total));
        builder.push_record(values);
    }

    if importer.len() > 1 {
        let grand_total: Decimal = account_totals.iter().sum();
        builder.push_record(
            once("TOTAL".to_string())
                .chain(repeat_n("".to_string(), fixed_columns.len() - 1))
                .chain(account_totals.iter().map(|n| money(*n)))
                .chain(once(money(grand_total))));
    }

    let mut table = builder.build();
    table.with((Alignment::right(), Padding::new(2, 2, 0, 0)));
    table.with(Style::modern_rounded());
    println!("{}", table);
}

fn valj_rapporter(rapporter: &Vec<RapportImport>) -> Vec<u32> {
    let mojliga: Vec<u32> = rapporter
        .iter()
        .filter(|e| e.verifikat.is_none())
        .map(|e| e.report.number)
        .collect();

    if mojliga.is_empty() {
        return Vec::new();
    }

    loop {
        print!("Importera ([J]a = alla*, [N]ej = ingen eller nummer)? ");
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        if let Ok(size) = std::io::stdin().read_line(&mut input) {
            if size == 0 {
                // EOF
                return Vec::new();
            }

            if input == "\n" {
                return mojliga;
            }

            input = input.trim().to_lowercase();
            if input.is_empty() {
                continue;
            }

            if input == "j" || input == "y" {
                return mojliga;
            }

            if input == "n" || input == "q" {
                return Vec::new();
            }

            let mut valda: Vec<u32> = Vec::new();
            for part in input.split_whitespace() {
                if let Ok(n) = part.parse::<u32>()
                    && mojliga.contains(&n)
                {
                    if !valda.contains(&n) {
                        valda.push(n);
                    }
                } else {
                    println!("Ogiltigt val: {}", part);
                    valda.clear();
                    break;
                }
            }

            if !valda.is_empty() {
                return valda;
            }
        }
    }
}

fn create_journal_entry(rapport: &SIEReportListItem) -> CreateJournal {
    let title = rapport.verifikatnamn();
    let date = rapport.datum();
    let mut items: Vec<CreateJournalAccount> =
        Vec::with_capacity(rapport.accounts.len());
    for tr in rapport.accounts.iter() {
        let debit = tr.amount.max(Decimal::ZERO);
        let credit = tr.amount.min(Decimal::ZERO).abs();
        let account = i32::from_str(&tr.number).unwrap();
        items.push(CreateJournalAccount {
            account,
            debit,
            credit,
        });
    }

    CreateJournal { title, date, items }
}

fn importera_rapport(
    kassa: &DinKassa,
    bokio: &Bokio,
    import: &RapportImport,
    save_files: bool,
) -> Result<JournalEntry, String> {
    println!(
        "Importerar Z-Rapport {}...",
        import.report.number
    );

    let basename = kassa.zreport_basename(&import.report);
    let pdf_filename = format!("{}.pdf", basename);
    let json_filename = format!("{}.json", basename);
    let sie4_filename = format!("{}.si", basename);

    print!("* Hämtar PDF... ");
    std::io::stdout().flush().ok();
    let pdf = kassa.zreport_pdf(&import.report.id).map_err(|e| {
        format!(
            "Kunde inte hämta PDF för Z-Rapport {}: {}",
            import.report.number, e
        )
    })?;

    println!("{}", pdf_filename);
    std::fs::write(&pdf_filename, pdf).expect("Kunde inte spara PDF.");

    print!("* Hämtar SIE4... ");
    std::io::stdout().flush().ok();
    let sie4 = kassa.zreport_sie(&import.report.id).map_err(|e| {
        format!(
            "Kunde inte hämta SIE4 för Z-Rapport {}: {}",
            import.report.number, e
        )
    })?;

    println!("{}", sie4_filename);
    std::fs::write(&sie4_filename, sie4).expect("Kunde inte spara SIE4.");

    let json = serde_json::to_vec_pretty(&import.sie).unwrap();
    print!("* Sparar {}...", json_filename);
    std::io::stdout().flush().ok();
    std::fs::write(&json_filename, json).expect("Kunde inte spara JSON.");

    let journal_entry = create_journal_entry(&import.sie);
    let bokio_json_filename = format!("{}.bokio.json", basename);
    print!(" {}", bokio_json_filename);
    std::io::stdout().flush().ok();
    let json = serde_json::to_vec_pretty(&journal_entry).unwrap();
    std::fs::write(&bokio_json_filename, json).expect("Kunde inte spara JSON.");
    println!();

    print!("* Bokför Z-Rapport {}... ", import.report.number);
    std::io::stdout().flush().ok();
    let journal_entry = bokio.create_journal_entry(&journal_entry).map_err(|e| {
        format!(
            "Kunde inte bokföra verifikat för Z-Rapport {}: {}",
            import.report.number, e
        )
    })?;
    println!("{}", journal_entry.journal_entry_number);

    print!("* Laddar upp underlag... ");
    std::io::stdout().flush().ok();
    bokio
        .upload(&pdf_filename, "application/pdf", &journal_entry.id)
        .inspect_err(|e| eprintln!("Misslyckades: {}", e))
        .inspect(|_| println!("OK"))
        .ok();

    if !save_files {
        for f in [&pdf_filename, &json_filename, &sie4_filename, &bokio_json_filename] {
            std::fs::remove_file(f)
                .inspect_err(|e| eprintln!("Misslyckades att radera: {}", e))
                .ok();
        }
    }

    println!();
    Ok(journal_entry)
}

fn importera(kassa: &DinKassa,
             bokio: &Bokio, rapporter: &mut Vec<RapportImport>,
             save_files: bool,
) {
    loop {
        lista_rapporter(&rapporter);
        let valda = valj_rapporter(&rapporter);
        if valda.is_empty() {
            break;
        }

        for seqnr in valda {
            let imp = rapporter
                .iter_mut()
                .find(|e| e.report.number == seqnr)
                .unwrap();
            println!();
            match importera_rapport(&kassa, &bokio, imp, save_files) {
                Ok(journal_entry) => {
                    imp.verifikat.replace(journal_entry);
                }
                Err(msg) => {
                    eprintln!("{}", msg);
                    break;
                }
            }
        }
    }
}

fn main() {
    let mut args = Cli {
        start_date: None,
        end_date: None,
        dinkassa_username: utils::get_env("DINKASSA_USERNAME"),
        dinkassa_password: utils::get_env("DINKASSA_PASSWORD"),
        bokio_api_url: utils::get_env_or_default("BOKIO_API_URL", BOKIO_API_URL),
        bokio_api_token: utils::get_env("BOKIO_API_TOKEN"),
        bokio_company_id: utils::get_env("BOKIO_COMPANY_ID"),
        save_files: false,
    };

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        if let Some(username) = check_arg("dinkassa-username", &arg, &mut iter) {
            args.dinkassa_username = username;
        } else if let Some(password) = check_arg("dinkassa-password", &arg, &mut iter) {
            args.dinkassa_password = password;
        } else if let Some(start) = check_arg("date", &arg, &mut iter).map(to_date) {
            args.start_date = Some(start);
            args.end_date = Some(start);
        } else if let Some(start) = check_arg("start", &arg, &mut iter).map(to_date) {
            args.start_date = Some(start);
        } else if let Some(end) = check_arg("end", &arg, &mut iter).map(to_date) {
            args.end_date = Some(end);
        } else if let Some(url) = check_arg("bokio-api-url", &arg, &mut iter) {
            args.bokio_api_url = url;
        } else if let Some(token) = check_arg("bokio-api-token", &arg, &mut iter) {
            args.bokio_api_token = token;
        } else if let Some(company_id) = check_arg("bokio-company-id", &arg, &mut iter) {
            args.bokio_company_id = company_id;
        } else if arg == "--save" || arg == "--save-files" {
            args.save_files = true;
        } else {
            eprintln!("{}: invalid option", arg);
            std::process::exit(1);
        }
    }

    if args.dinkassa_username.is_empty() {
        let username = read_prompt_trim("dinkassa.se username: ");
        if username.is_empty() {
            return;
        }
        args.dinkassa_username = username;
    }

    if args.dinkassa_password.is_empty() {
        let password = read_password_trim("dinkassa.se password: ");
        if password.is_empty() {
            return;
        }
        args.dinkassa_password = password;
    }

    if args.bokio_api_token.is_empty() {
        let token = read_password_trim("Bokio API token: ");
        if token.is_empty() {
            return;
        }
        args.bokio_api_token = token;
    }

    if args.bokio_company_id.is_empty() {
        let company_id = read_prompt_trim("Bokio company id: ");
        if company_id.is_empty() {
            return;
        }
        args.bokio_company_id = company_id;
    }

    let kassa = DinKassa::login_username_password(
        &args.dinkassa_username,
        &args.dinkassa_password
    );
    let kassa = kassa
        .inspect_err(|err| {
            eprintln!("Inloggning på dinkassa.se misslyckades: {}", err);
            std::process::exit(1);
        })
        .unwrap();

    let bokio = Bokio::new(
        &args.bokio_api_url,
        &args.bokio_company_id,
        &args.bokio_api_token,
    );

    let (mut rapporter, dates) = hamta_rapporter(&args, &kassa, &bokio)
        .inspect_err(|err| {
            eprintln!("Kunde inte hämta Z-Rapporter: {}", err);
            std::process::exit(1);
        })
        .unwrap();

    println!(
        "{} Z-Rapporter för {} ({} - {})",
        rapporter.len(),
        kassa.machine.customer_name,
        format_local_date(&dates.start_date),
        format_local_date(&dates.end_date),
    );

    if !rapporter.is_empty() {
        let antal_skippade = rakna_importerade_rapporter(&rapporter);
        importera(&kassa, &bokio, &mut rapporter, args.save_files);
        let antal_importerade = rakna_importerade_rapporter(&rapporter) - antal_skippade;

        println!();
        println!("{} Z-Rapporter importerades", antal_importerade);
        if antal_skippade > 0 {
            println!("{} Z-Rapporter redan importerade", antal_skippade);
        }
    }
}
