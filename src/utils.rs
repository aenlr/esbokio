use std::io::{IsTerminal, Write};
use std::str::FromStr;
use chrono::{Datelike, NaiveDate};
use chrono::naive::Days;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

#[derive(Debug)]
pub struct PageReq {
    pub page: u32,
    pub size: u32,
}

pub fn format_local_date(date: &NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:140.0) Gecko/20100101 Firefox/140.0";
pub const APPLICATION_JSON: &'static str = "application/json";

fn read_prompt(prompt: &str) -> std::io::Result<String> {
    print!("{}", prompt);
    std::io::stdout().flush().and_then(|_| {
        let mut val = String::new();
        match std::io::stdin().read_line(&mut val) {
            Ok(_) => Ok(val.trim().to_string()),
            Err(e) => Err(e),
        }
    })
}

pub fn read_prompt_trim(prompt: &str) -> String {
    read_prompt(prompt).unwrap().trim().to_string()
}

fn read_password(prompt: &str) -> std::io::Result<String> {
    // IntelliJ console is broken giving "device not ready" for /dev/tty.
    // Strangely the builtin terminal works fine.
    if std::io::stdin().is_terminal() && !std::env::var("BROKEN_TERMINAL").is_ok() {
        rpassword::prompt_password(prompt)
    } else {
        read_prompt(prompt)
    }
}

pub fn read_password_trim(prompt: &str) -> String {
    read_password(prompt).unwrap().trim().to_string()
}

pub fn to_date(s: String) -> NaiveDate {
    if s == "today" || s == "0" {
        chrono::Local::now().date_naive()
    } else if s == "yesterday" {
        chrono::Local::now().date_naive().pred_opt().unwrap()
    } else if s == "week" {
        let now = chrono::Local::now().date_naive();
        let days = Days::new(now.weekday().num_days_from_monday().to_u64().unwrap());
        now.checked_sub_days(days).unwrap()
    } else if s == "month" || s == "first" {
        chrono::Local::now().date_naive().with_day(1).unwrap()
    } else if s.starts_with("-") && s.chars().skip(1).all(|c| c.is_digit(10)) {
        let days = u64::from_str(&s[1..]).unwrap();
        chrono::Local::now().date_naive().checked_sub_days(Days::new(days)).unwrap()
    } else {
        NaiveDate::from_str(&s).unwrap()
    }
}

pub fn money(n: Decimal) -> String {
    format!("{:.2}", n)
}

pub fn get_env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or(default.into())
}

pub fn get_env(key: &str) -> String {
    get_env_or_default(key, "")
}
