#![feature(proc_macro_hygiene, decl_macro)]
#![feature(str_split_as_str)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate lazy_static;

use std::collections::HashMap;
use std::path::{PathBuf, Path};

use serde::Serialize;
use urlencoding::decode;
use url::Url;

use rocket::http::RawStr;
use rocket::http::hyper::header::Location;
use rocket_contrib::templates::{Template};
use rocket::fs::{NamedFile, relative};

use scraper::{Html,Selector};

#[derive(Responder)]
#[response(status=303)]
struct RawRedirect((), Location);

#[derive(Responder)]
enum ExampleResponse {
    Template(Template),
    Redirect(RawRedirect),
}

#[derive(Debug, Serialize)]
struct TemplateContext {
    query: String,
    results: Vec<ContextResult>
}
#[derive(Debug, Serialize)]
struct ContextResult {
    title: String,
    link: String,
    desc: String
}
lazy_static! {
    static ref BANGS: HashMap<&'static str,&'static str> = [
        ("tr", "https://translate.google.com/#auto/en/{}"),
        ("r", "https://www.reddit.com/search?q={}"),
        ("rio","https://raider.io/search?type=character&name[0][contains]={}"),
        ("yt" ,"https://www.youtube.com/results?search_query={}"),
        ("gh" ,"https://github.com/search?q={}"),
        ("tw" ,"https://twitter.com/search?q={}"),
        ("m" ,"https://www.google.no/maps?q={}"),
        ("imdb","https://www.imdb.com/find?s=all&q={}")
    ].iter().cloned().collect();

}
static DDG_SEARCH: &'static str="http://duckduckgo.com/?q=";


fn get_bang(bang:&str, r:&str) -> String {
    let url: String;
    if BANGS.contains_key(bang) {
        url=BANGS.get(bang).unwrap().replace("{}",r);
    }else{
       return format!("{}!{} {}",DDG_SEARCH,bang,r)
    }
    if r=="" {
        let url=Url::parse(&url).unwrap();
        format!("{}://{}",url.scheme(), url.host_str().unwrap())
    }else{
        url
    }
}
fn handle_bang(q: String) -> ExampleResponse {
    let _q=&q[1..];
    let s=&mut _q.split(" ");
    let bang=s.next().unwrap();
    let st = s.as_str();
    let b=get_bang(bang,st);
    ExampleResponse::Redirect(RawRedirect((),Location(b)))
}
fn handle_query(q: String) -> ExampleResponse {
    let html: String = ureq::get(&format!("https://html.duckduckgo.com/html?q={}",q))
        .call().unwrap()
        .into_string().unwrap();
    let fragment = Html::parse_fragment(&html);
    let web_result = Selector::parse(".web-result").unwrap();
    let title = Selector::parse(".result__title a").unwrap();
    let link = Selector::parse(".result__url").unwrap();
    let desc = Selector::parse(".result__snippet").unwrap();

    let results=fragment.select(&web_result).take(20)
        .filter_map(|e| {
            let link = e.select(&link).next();
            let result_link;
            if link == None{
                return None;
            }else{
                result_link=link.unwrap().inner_html().trim().to_string();
            }
            let desc = e.select(&desc).next();
            let result_desc;
            if desc == None{
                return None;
            }else{
                result_desc=desc.unwrap().text().collect::<String>()
            }
            return Some(ContextResult{
                title: e.select(&title).next().unwrap().inner_html(),
                link: result_link,
                desc: result_desc
            })
        })
        .collect::<Vec<_>>();
    ExampleResponse::Template(Template::render("index",&TemplateContext{
        query: q.to_string(),
        results: results
    }))
}

#[get("/?<q>")]
fn query(q: &RawStr) -> ExampleResponse {
    let q=q.replace("+"," ");
    let q=decode(&q).unwrap_or(q.to_string());
    println!("{}", q);
    if q.starts_with("!") {
        handle_bang(q)
    }else{
        handle_query(q)
    }
}

#[get("/favicon.ico")]
fn favicon() -> Option<NamedFile> {
    NamedFile::open("favicon.ico").await.ok()
}

#[catch(404)]
fn not_found() -> String {
    String::from("Kys")
}

fn main() {
    rocket::ignite()
        .mount("/", routes![query])
        .attach(Template::fairing())
        .launch();
}
