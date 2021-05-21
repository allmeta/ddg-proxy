#![feature(proc_macro_hygiene, decl_macro)]
#![feature(str_split_as_str)]

#[macro_use] extern crate rocket;

use serde::Serialize;
use urlencoding::decode;

use rocket::http::RawStr;
use rocket::http::hyper::header::Location;
use rocket_contrib::templates::{Template};

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


fn get_bang(bang:&str, r:&str) -> String {
    let b: &str="http://duckduckgo.com/?q=!";
    let default: &str = &[b,bang,&" {}"].concat();
    match bang {
      "tr"  => "https://translate.google.com/#auto/en/{}",
      "r"   => "https://www.reddit.com/search?q={}",
      "rio" => "https://raider.io/search?type=character&name[0][contains]={}",
      "yt"  => "https://www.youtube.com/results?search_query={}",
      "gh"  => "https://github.com/search?q={}",
      "tw"  => "https://twitter.com/search?q={}",
      "imdb"=> "https://www.imdb.com/find?s=all&q={}",
        _   => default
    }.replace("{}",r)
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

    let results=fragment.select(&web_result).take(10)
        .map(|e| 
            ContextResult{
                title: e.select(&title).next().unwrap().inner_html(),
                link: e.select(&link).next().unwrap().inner_html().trim().to_string(),
                desc:e.select(&desc).next().unwrap().text().collect::<String>(),
            }
        )
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
