extern crate hyper;
extern crate futures;
extern crate tokio_core;
extern crate hyper_tls;
extern crate scraper;


use hyper::{Response, Client};
use futures::{Future, Stream};
use scraper::{Html, Selector};
use std::borrow::Cow;
use futures::stream::Concat2;
use tokio_core::reactor::Core;
use std::time::Instant;
use std::thread::sleep;


fn get_html_str<Connector>(hyper_client: &Client<Connector>, url: &str) -> Box<Future<Item=String, Error=hyper::error::Error>>
    where Connector : hyper::client::Connect {
    let html_response_fut = hyper_client.get(url.parse().expect("Failed to parse url"))
        .and_then(|resp| resp.body().concat2())
        .map(|concat2| concat2.into_iter())
        .map(|iter| {
            println!("Request completed");
            let utf_8_bytes = iter.collect::<Vec<u8>>();
            String::from_utf8_lossy(&utf_8_bytes).into_owned()
        });
    Box::new(html_response_fut)
}


fn main() {


    let mut core = tokio_core::reactor::Core::new().unwrap();
    let client = hyper::Client::configure()
        .connector(hyper_tls::HttpsConnector::new(10, &core.handle()).unwrap())
        .build(&core.handle());

    let page_comments_futures = get_html_str(&client, "https://www.reddit.com/r/programming/")
        .map(|resolved_html:String| {
            let doc = Html::parse_document(&resolved_html);
            let posts_comment_urls_sel = Selector::parse("div#siteTable > div.thing a.comments").unwrap();
            let urls:Vec<String> = doc.select(&posts_comment_urls_sel).filter_map(|el| el.value().attr("href").map(|s| s.to_owned())).collect();
            println!("Gathered {:?} urls", urls.len());
            urls
        }).map(|urls| {
            let comment_futures = urls.into_iter().map(|owned_url| {
                get_html_str(&client, &owned_url).map(|owned_html| {
                    let doc = Html::parse_document(&owned_html);

                    let comment_text_sel = Selector::parse("div.comment div.md > p").unwrap();
                    let comments_text:Vec<String> = doc.select(&comment_text_sel).filter_map(|el| el.text().next().map(|s| s.to_owned())).collect();

                    comments_text
                })
        });
        comment_futures.collect::<Vec<_>>()
    });
    let start = Instant::now();

    let little_futures = core.run(page_comments_futures).unwrap();
    let big_ass_future = futures::future::join_all(little_futures);

    let results = core.run(big_ass_future).unwrap();


    let all_comments = results.into_iter().flat_map(|x| x).collect::<Vec<_>>();
    let elapsed = start.elapsed();

    println!("TOTAL TIME = {:?}", elapsed);
    println!("Number of comments scraped {:?}", all_comments.len());



}