use crate::utils::{format_local_date, APPLICATION_JSON, DEFAULT_USER_AGENT};
use chrono::NaiveDate;
use http::header::{ACCEPT, USER_AGENT};
use http::{HeaderValue, Request, Response};
use regex::Regex;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;
use ureq::middleware::{Middleware, MiddlewareNext};
use ureq::{Agent, Body, Error, SendBody};
use urlencoding::encode;

pub const DINKASSA_API_URL: &str = "https://www.dinkassa.se/api";

const SESSION_ID: &str = "SessionId";
const INTEGRATOR_ID: &str = "IntegratorId";
//const MACHINE_ID: &str = "MachineId";
//const MACHINE_KEY: &str = "MachineKey";

const WEB_INTEGRATOR_ID: &str = "cc7c4035-ce21-40a6-95e2-a39a641a1c27";

#[derive(Debug)]
pub struct DateRange {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

impl DateRange {
    pub fn new(start_date: &Option<NaiveDate>, end_date: &Option<NaiveDate>) -> Self {
        let today = chrono::Local::now().date_naive();
        let (start_date, end_date) = {
            if let Some(start_date) = start_date {
                if let Some(end_date) = end_date {
                    (*start_date, *end_date)
                } else {
                    (*start_date, today)
                }
            } else if let Some(end_date) = end_date {
                if today < *end_date {
                    (today, *end_date)
                } else {
                    (*end_date, *end_date)
                }
            } else {
                (today, today)
            }
        };

        Self {
            start_date,
            end_date,
        }
    }
}

#[derive(Debug)]
pub struct DinKassa {
    base_url: String,
    agent: Agent,
    pub machine: Machine,
}

#[derive(Deserialize)]
pub struct WebLoginResponse {
    #[serde(rename = "Id")]
    pub id: String, // sessionid header
    #[serde(rename = "Type")]
    pub _type: u32, // 2
    #[serde(rename = "WebUserId")]
    pub web_user_id: String,
    #[serde(rename = "ExpiresDateTime")]
    pub expires_date_time: String,
}

#[derive(Deserialize)]
pub struct SettingsResponse {
    #[serde(rename = "Unit")]
    pub unit: String, // default unit
    #[serde(rename = "ControlUnitSerialNumber")]
    pub control_unit_serial_number: String,
    #[serde(rename = "MachineId")]
    pub machine_id: String,
}

#[derive(Deserialize)]
#[derive(Clone, Debug)]
pub struct Machine {
    //#[serde(rename = "LastConnection")]
    //pub last_connection: String,
    #[serde(rename = "CustomerName")]
    pub customer_name: String,
    //#[serde(rename = "ComputerName")]
    //pub computer_name: String,
    //#[serde(rename = "Version")]
    //pub version: String,
    #[serde(rename = "Name")]
    pub name: String,
    //#[serde(rename = "CreatedTime")]
    //pub created_time: String,
    //#[serde(rename = "LicenseNumber")]
    //pub license_numer: String,
    #[serde(rename = "Id")]
    pub id: String,
}

#[derive(Deserialize)]
pub struct MachineResponse {
    //#[serde(rename = "ItemCountFetched")]
    //pub item_count_fetched: u32,
    #[serde(rename = "Items")]
    pub items: Vec<Machine>,
}

#[derive(Clone, Debug)]
#[derive(Deserialize)]
pub struct ZReportListItem {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Number")]
    pub number: u32,
    #[serde(rename = "DateTime")]
    pub date_time: String,
    #[serde(rename = "CreatedBy")]
    pub created_by: String,
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct ZReportListResponse {
    //#[serde(rename = "ItemCountFetched")]
    //pub item_count_fetched: u32,
    #[serde(rename = "Items")]
    pub items: Vec<ZReportListItem>,
}

//             "ZReport": "K1:1",
//             "ReportDateTime": "2026-01-04T17:30:26",
//             "FirstTransactionDateTime": "2026-01-04T11:10:12",
//             "LastTransactionDateTime": "2026-01-04T15:46:34",

#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct SIEReportAccount {
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "Number")]
    pub number: String,
    #[serde(with = "rust_decimal::serde::float")]
    #[serde(rename = "Amount")]
    pub amount: Decimal,
}

#[derive(Debug)]
#[derive(Deserialize, Serialize)]
pub struct SIEReportListItem {
    #[serde(rename = "ZReport")]
    pub zreport: String,
    #[serde(rename = "ReportDateTime")]
    pub report_date_time: String,
    #[serde(rename = "FirstTransactionDateTime")]
    pub first_transaction_date_time: String,
    #[serde(rename = "LastTransactionDateTime")]
    pub last_transaction_date_time: String,
    #[serde(rename = "Accounts")]
    pub accounts: Vec<SIEReportAccount>,
}

impl SIEReportListItem {
    pub fn verifikatnamn(&self) -> String {
        let re = Regex::new(r"K(\d+):(\d+)").unwrap();
        if let Some(captures) = re.captures(&self.zreport) {
            format!("Kassa {}, Z-Rapport #{}", &captures[1], &captures[2])
        } else {
            self.zreport.to_string()
        }
    }

    pub fn number(&self) -> Result<u32, ParseIntError> {
        let re = Regex::new(r"K\d+:(\d+)").unwrap();
        if let Some(captures) = re.captures(&self.zreport) {
            u32::from_str(&captures[1])
        } else {
            u32::from_str(&self.zreport)
        }
    }

    pub fn datum(&self) -> String {
        self.report_date_time[0..10].to_string()
    }

    pub fn konto(&self, nr: &str) -> Option<Decimal> {
        self.accounts.iter().find_map(|a| if a.number == nr { Some(a.amount) } else { None })
    }
}

#[derive(Debug)]
#[derive(Deserialize)]
pub struct SIEReportListResponse {
    #[serde(rename = "ZReports")]
    pub zreports: Vec<SIEReportListItem>,
}

impl Display for SIEReportListResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

fn start_of_day(date: &NaiveDate) -> String {
    date.format("%Y-%m-%dT00:00:00").to_string()
}

fn end_of_day(date: &NaiveDate) -> String {
    date.format("%Y-%m-%dT23:59:59").to_string()
}

struct WebUserSession {
    integrator_id: String,
    session_id: String,
}

impl Middleware for WebUserSession {
    fn handle(&self, mut req: Request<SendBody>, next: MiddlewareNext)
              -> Result<Response<Body>, ureq::Error> {

        req.headers_mut()
            .insert(INTEGRATOR_ID, HeaderValue::from_str(&self.integrator_id).unwrap());
        if !self.session_id.is_empty() {
            req.headers_mut()
                .insert(SESSION_ID, HeaderValue::from_str(&self.session_id).unwrap());
        }

        // continue the middleware chain
        next.handle(req)
    }
}


impl DinKassa {
    pub fn login_username_password(
        username: &str,
        password: &str,
    ) -> Result<DinKassa, Error> {
        let base_url = DINKASSA_API_URL;
        let integrator_id = WEB_INTEGRATOR_ID.to_string();
        let url = format!("{}/session/Authenticate?type=2", base_url);
        let mut body = std::collections::HashMap::new();
        body.insert("Username", username);
        body.insert("Password", password);

        let agent: Agent = Agent::config_builder()
            .accept(APPLICATION_JSON)
            .user_agent(DEFAULT_USER_AGENT)
            .build()
            .into();
        let res = agent.post(url)
            .header(INTEGRATOR_ID, &integrator_id)
            .send_json(&body)?
            .body_mut()
            .read_json::<WebLoginResponse>()?;
        let session = WebUserSession {
            integrator_id,
            session_id: res.id
        };
        let jar = agent.cookie_jar_lock();
        let mut buf = Vec::new();
        jar.save_json(&mut buf)?;
        jar.release();

        let agent: Agent = Agent::config_builder()
            .accept(APPLICATION_JSON)
            .user_agent(USER_AGENT)
            .middleware(session)
            .build()
            .into();
        let mut jar = agent.cookie_jar_lock();
        jar.load_json(&buf[..])?;
        jar.release();

        let settings = agent.get(format!("{}/settings", base_url))
            .call()?
            .body_mut()
            .read_json::<SettingsResponse>()?;

        let machine_id = settings.machine_id;
        let machine = agent.get(format!("{}/machine", base_url))
            .call()?
            .body_mut()
            .read_json::<MachineResponse>()?
            .items
            .iter().find_map(|m| if m.id == machine_id { Some(m.clone()) } else { None })
            .unwrap_or(Machine {
                customer_name: "".to_string(),
                id: machine_id,
                name: "kassa".to_string()
            });


        Ok(DinKassa {
            agent,
            base_url: base_url.to_string(),
            machine,
        })
    }

    pub fn list_zreports(&self, dates: &DateRange) -> Result<ZReportListResponse, Error> {
        let url = format!("{}/reports/get-z-reports?machineId={}&startDateTime={}&endDateTime={}",
                          self.base_url, self.machine.id,
                          encode(&start_of_day(&dates.start_date)),
                          encode(&end_of_day(&dates.end_date)));

        self.agent.get(url)
            .call()?
            .body_mut()
            .read_json::<ZReportListResponse>()
    }

    pub fn list_sie_reports(&self, dates: &DateRange) -> Result<SIEReportListResponse, Error> {
        let url = format!("{}/reports/download-z-report-by-date/json?machineId={}&startDate={}&endDate={}",
                          self.base_url, self.machine.id,
                          format_local_date(&dates.start_date),
                          format_local_date(&dates.end_date));

        self.agent.get(url)
            .call()?
            .body_mut()
            .read_json::<SIEReportListResponse>()
    }

    pub fn zreport_pdf(&self, report_id: &str) -> Result<Vec<u8>, Error> {
        let url = format!("{}/reports/download-z-report/{}/{}",
            self.base_url, self.machine.id, report_id);

        self.agent.get(url)
            .header(ACCEPT, "application/pdf")
            //.header(ACCEPT, "application/json")
            //.header(ACCEPT, "text/plain")
            //.header(ACCEPT, "*/*")
            .call()?
            .body_mut()
            .read_to_vec()
    }

    pub fn zreport_sie(&self, report_id: &str) -> Result<Vec<u8>, Error> {
        let url = format!("{}/reports/download-z-report/{}/{}/sie4",
                          self.base_url, self.machine.id, report_id);

        self.agent.get(url)
            .header(ACCEPT, "text/plain")
            .header(ACCEPT, "*/*")
            .call()?
            .body_mut()
            .read_to_vec()
    }

    pub fn zreport_basename(&self, report: &ZReportListItem) -> String {
        if self.machine.customer_name.is_empty() {
            format!("Z{}_{}", report.number, self.machine.name)
        } else {
            format!("Z{}_{}_{}", report.number, self.machine.name, self.machine.customer_name)
        }
    }

}
