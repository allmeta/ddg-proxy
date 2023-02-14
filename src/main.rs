#![feature(proc_macro_hygiene, decl_macro)]
#![feature(str_split_as_str)]
#![feature(option_result_contains)]
#![feature(once_cell)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate rocket;

use either::{Either, Left, Right};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::sync::Mutex;

use serde::Serialize;
use url::Url;
use urlencoding::{decode, encode};

use rocket::fs::NamedFile;
use rocket::response::Redirect;
use rocket_dyn_templates::Template;

use scraper::{Html, Selector};

#[derive(Debug, Serialize)]
struct TemplateContext {
    query: String,
    backend: String,
    results: Vec<ContextResult>,
}
#[derive(Debug, Serialize)]
struct ContextResult {
    title: String,
    link: String,
    desc: String,
}
lazy_static! {
    static ref BANGS: Mutex<HashMap<String, String>> = {
        let m = fs::read_to_string(env::var("DP_BANGS").unwrap())
            .expect("Could not find bangs file")
            .split("\n")
            .filter_map(|x| {
                if x == "" {
                    return None;
                }
                let mut x = x.split(" ");
                let a = x.nth(0).unwrap().to_string();
                let b = x.nth(0).unwrap().to_string();
                return Some((a, b));
            })
            .collect();
        Mutex::new(m)
    };
    static ref SELECTORS: HashMap<&'static str, [&'static str; 4]> = [
        (
            "ddg",
            [
                ".web-result",
                ".result__title a",
                ".result__url",
                ".result__snippet"
            ]
        ),
        (
            "google",
            [
                "div.g",
                ".LC20lb.MBeuO.DKV0Md",
                ".yuRUbf a",
                ".VwiC3b.yXK7lf.MUxGbd.yDYNvb.lyLwlc.lEBKkf"
            ]
        )
    ]
    .iter()
    .cloned()
    .collect();
}
static DDG_URL: &'static str = "https://duckduckgo.com/?q=";
static DDG_HTML_URL: &'static str = "https://html.duckduckgo.com/html?q=";
static GOOGLE_URL: &'static str = "https://www.google.com/search?q=";

fn refresh_bangs() {
    let mut map = BANGS.lock().unwrap();
    let m: HashMap<String, String> = fs::read_to_string(env::var("DP_BANGS").unwrap())
        .expect("Could not find bangs file")
        .split("\n")
        .filter_map(|x| {
            if x == "" {
                return None;
            }
            let mut x = x.split(" ");
            let a = x.next().unwrap().to_string();
            let b = x.next().unwrap().to_string();
            return Some((a, b));
        })
        .collect();
    map.extend(m)
}

fn get_bang(bang: &str, r: &str) -> String {
    let url: String;
    let map = BANGS.lock().unwrap();
    if map.contains_key(bang) {
        url = map.get(bang).unwrap().replace("{}", &encode(&r));
    } else {
        return format!("{}!{}%20{}", DDG_URL, bang, encode(&r));
    }
    if r == "" {
        let url = Url::parse(&url).unwrap();
        return format!("{}://{}", url.scheme(), url.host_str().unwrap());
    } else {
        url
    }
}

fn handle_bang(q: String) -> Redirect {
    let (bang, rest): (Vec<&str>, Vec<&str>) = q.split(" ").partition(|n| n.starts_with("!"));
    let bang = &bang.get(0).unwrap()[1..];
    let rest = &rest.join(" ");
    let b = get_bang(&bang, rest);
    Redirect::to(b)
}

fn handle_ddg_query(q: String) -> Template {
    let html: String = ureq::get(&format!("{}{}", DDG_HTML_URL, q))
        .call()
        .unwrap()
        .into_string()
        .unwrap();
    let fragment = Html::parse_fragment(&html);
    let web_result = Selector::parse(".web-result").unwrap();
    let title = Selector::parse(".result__title a").unwrap();
    let link = Selector::parse(".result__url").unwrap();
    let desc = Selector::parse(".result__snippet").unwrap();

    let results = fragment
        .select(&web_result)
        .take(20)
        .filter_map(|e| {
            let link = e.select(&link).next();
            if link == None {
                return None;
            }
            let result_link = link.unwrap().inner_html().trim().to_string();
            let result_link = format!("https://{}", result_link);

            let desc = e.select(&desc).next();
            if desc == None {
                return None;
            }
            let result_desc = desc.unwrap().text().collect::<String>();

            let title = e.select(&title).next();
            if title == None {
                return None;
            }
            let result_title = title.unwrap().inner_html();

            return Some(ContextResult {
                title: result_title,
                link: result_link,
                desc: result_desc,
            });
        })
        .collect::<Vec<_>>();
    Template::render(
        "index",
        &TemplateContext {
            query: q.to_string(),
            backend: "ddg".to_string(),
            results: results,
        },
    )
}
fn handle_google_query(q: String) -> Template {
    let html: String = ureq::get(&format!("{}{}", GOOGLE_URL,q))
        .set("authority", "www.google.com")
        .set("user-agent", "user-agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) QtWebEngine/5.15.2 Chrome/87.0.4280.144 Safari/537.36")
        .call().unwrap()
        .into_string().unwrap();
    let fragment = Html::parse_fragment(&html);
    let web_result = Selector::parse("div.g").unwrap();
    let title = Selector::parse(".LC20lb").unwrap();
    let link = Selector::parse(".yuRUbf a").unwrap();
    let desc = Selector::parse(".VwiC3b").unwrap();

    let results = fragment
        .select(&web_result)
        .take(20)
        .filter_map(|e| {
            let link = e.select(&link).next();
            if link == None {
                return None;
            }
            let result_link = link.unwrap().value().attr("href").unwrap().to_string();

            let desc = e.select(&desc).next();
            if desc == None {
                return None;
            }
            let result_desc = desc.unwrap().text().collect::<String>();

            let title = e.select(&title).next();
            if title == None {
                return None;
            }
            let result_title = title.unwrap().inner_html();

            return Some(ContextResult {
                title: result_title,
                link: result_link,
                desc: result_desc,
            });
        })
        .collect::<Vec<_>>();
    Template::render(
        "index",
        &TemplateContext {
            query: q.to_string(),
            backend: "google".to_string(),
            results: results,
        },
    )
}

#[get("/?<q>&<b>")]
fn query(q: String, b: Option<String>) -> Either<Redirect, Template> {
    let q = q.replace("+", " ");
    let q = decode(&q).unwrap_or(q.to_string());
    let b = b.unwrap_or(String::from("ddg"));
    if q.contains("!") {
        Left(handle_bang(q))
    } else if b.contains("google") {
        Right(handle_google_query(q))
    } else {
        Right(handle_ddg_query(q))
    }
}
#[get("/favicon.ico")]
async fn favicon() -> Option<NamedFile> {
    NamedFile::open("favicon.ico").await.ok()
}
#[post("/refresh")]
async fn refresh() -> String {
    refresh_bangs();
    return String::from("Success!");
}

#[launch]
fn rocket() -> _ {
    match env::var("DP_BANGS") {
        Ok(val) => println!("DP_BANGS: {}", val),
        Err(e) => panic!("DP_BANGS not set. Should point to the bangs file.: {}", e),
    };
    rocket::build()
        .mount("/", routes![query, favicon, refresh])
        .attach(Template::fairing())
}
