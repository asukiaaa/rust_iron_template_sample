extern crate iron;
extern crate params;
#[macro_use] extern crate router;
extern crate time;

use handlebars_iron as hbs;
use hbs::{DirectorySource, HandlebarsEngine, Template};
use iron::prelude::*;
use iron::status;
use iron::{typemap, AfterMiddleware, BeforeMiddleware};
use mount::Mount;
use params::{Params, Value};
use router::Router;
use staticfile::Static;
use std::collections::HashMap;
use std::path::Path;
use time::precise_time_ns;

struct ResponseTime;

impl typemap::Key for ResponseTime { type Value = u64; }

impl BeforeMiddleware for ResponseTime {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<ResponseTime>(precise_time_ns());
        Ok(())
    }
}

impl AfterMiddleware for ResponseTime {
    fn after(&self, req: &mut Request, res: Response) -> IronResult<Response> {
        let delta = precise_time_ns() - *req.extensions.get::<ResponseTime>().unwrap();
        println!("Request took: {} ms", (delta as f64) / 1000000.0);
        Ok(res)
    }
}

fn create_default_data() -> HashMap<String, String> {
    HashMap::new()
}

fn root_handler(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    let mut data = create_default_data();
    data.insert("hello_url".to_string(), url_for!(req, "hello").to_string());
    data.insert("hello_again_url".to_string(), url_for!(req, "hello_again").to_string());
    data.insert(
        "hello_again_bob_url".to_string(),
        url_for!(req, "hello_again", "name" => "Bob").to_string()
    );
    resp.set_mut(Template::new("index", data)).set_mut(status::Ok);
    Ok(resp)
}

fn hello_handler(_: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    let data = create_default_data();
    resp.set_mut(Template::new("hello", data)).set_mut(status::Ok);
    Ok(resp)
}

fn hello_again_handler(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    let mut data = create_default_data();
    let params = req.get_ref::<Params>().unwrap();
    match params.find(&["name"]) {
        Some(&Value::String(ref name)) => {
            data.insert("name".to_string(), name.to_string());
        },
        _ => {}
    };
    resp.set_mut(Template::new("hello_again", data)).set_mut(status::Ok);
    Ok(resp)
}

fn main() {
    let mut router = Router::new();
    let mut hbse = HandlebarsEngine::new();
    hbse.add(Box::new(DirectorySource::new("./templates", ".hbs")));
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }

    router.get("/".to_string(), root_handler, "root");
    router.get("/hello".to_string(), hello_handler, "hello");
    router.get("/hello/again".to_string(), hello_again_handler, "hello_again");
    router.get("/error".to_string(), |_: &mut Request| {
        Ok(Response::with(status::BadRequest))
    }, "error");

    let mut mount = Mount::new();
    mount
        .mount("/", router)
        .mount("/public", Static::new(Path::new("public")));

    let mut chain = Chain::new(mount);
    chain.link_before(ResponseTime);
    chain.link_after(ResponseTime);
    chain.link_after(hbse);
    if let Err(r) = Iron::new(chain).http("localhost:3000") {
        panic!("{}", r);
    }
}
