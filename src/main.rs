use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use actix_web::web::Bytes;
use quick_xml::events::{BytesDecl, BytesStart, BytesEnd, BytesText, BytesPI, Event};
use quick_xml::events::attributes::Attribute;
use quick_xml::Writer;
use std::io::Cursor;

trait XmlContainer<W: std::io::Write> {
    fn writer(&mut self) -> &mut Writer<W>;
    fn tag<'a>(&'a mut self, tag_name: &str) -> XmlTag<'a, W>;
    fn tag_with_attrs<'a, I>(&'a mut self, tag_name: &str, attributes: I) -> XmlTag<'a, W>
    where
        I: IntoIterator<Item = Attribute<'a>>;
    fn text(&mut self, text: &str) -> &mut Self;
    fn pi(&mut self, text: &str) -> &mut Self;
}

struct XmlTag<'a, W: std::io::Write> {
    writer_: &'a mut quick_xml::Writer<W>,
    tag_name: String,
}

impl <'a, W: std::io::Write> XmlTag<'a, W> {
    pub fn new(parent: &'a mut quick_xml::Writer<W>, tag_name: &str) -> Self {
        parent.write_event(Event::Start(BytesStart::new(tag_name))).unwrap();
        XmlTag { writer_: parent, tag_name: tag_name.to_string() }
    }

    pub fn new_with_attrs<'b, I, A>(parent: &'a mut quick_xml::Writer<W>, tag_name: &str, attributes: I) -> Self
    where
        I: IntoIterator<Item = A>,
        A: Into<Attribute<'b>>,
    {
        let start = BytesStart::new(tag_name).with_attributes(attributes);
        parent.write_event(Event::Start(start)).unwrap();
        XmlTag { writer_: parent, tag_name: tag_name.to_string() }
    }
}

impl<W: std::io::Write> Drop for XmlTag<'_, W> {
    fn drop(&mut self) {
        self.writer_.write_event(Event::End(BytesEnd::new(self.tag_name.as_str()))).unwrap();
    }
}

impl<'tag, W: std::io::Write> XmlContainer<W> for XmlTag<'tag, W> {
    fn writer(&mut self) -> &mut Writer<W> {
        self.writer_
    }

    fn tag<'a>(&'a mut self, tag_name: &str) -> XmlTag<'a, W> {
        XmlTag::new(self.writer_, tag_name)
    }

    fn tag_with_attrs<'a, I>(&'a mut self, tag_name: &str, attributes: I) -> XmlTag<'a, W>
    where
        I: IntoIterator<Item = Attribute<'a>>,
    {
        XmlTag::new_with_attrs(self.writer_, tag_name, attributes)
    }

    fn text(&mut self, text: &str) -> &mut Self {
        self.writer_.write_event(Event::Text(BytesText::new(text))).unwrap();
        self
    }

    fn pi(&mut self, text: &str) -> &mut Self {
        self.writer_.write_event(Event::PI(BytesPI::new(text))).unwrap();
        self
    }
}

struct XmlDocument {
    writer_: Writer<Cursor<Vec<u8>>>,
}

impl XmlDocument {
    fn new() -> Self {
        let mut writer = Writer::new(Cursor::new(Vec::<u8>::new()));
        writer
            .write_event(Event::Decl(BytesDecl::new(
                "1.0",
                Some("UTF-8"),
                Some("yes"),
            )))
            .unwrap();
        Self { writer_: writer }
    }

    fn writer(&mut self) -> &mut Writer<Cursor<Vec<u8>>> {
        &mut self.writer_
    }
}

impl XmlContainer<Cursor<Vec<u8>>> for XmlDocument {
    fn writer(&mut self) -> &mut Writer<Cursor<Vec<u8>>> {
        &mut self.writer_
    }

    fn tag<'a>(&'a mut self, tag_name: &str) -> XmlTag<'a, Cursor<Vec<u8>>> {
        XmlTag::new(&mut self.writer_, tag_name)
    }

    fn tag_with_attrs<'a, I>(&'a mut self, tag_name: &str, attributes: I) -> XmlTag<'a, Cursor<Vec<u8>>> 
    where
        I: IntoIterator<Item = Attribute<'a>>,
    {
        XmlTag::new_with_attrs(&mut self.writer_, tag_name, attributes)
    }

    fn text(&mut self, text: &str) -> &mut Self {
        self.writer_.write_event(Event::Text(BytesText::new(text))).unwrap();
        self
    }

    fn pi(&mut self, text: &str) -> &mut Self {
        self.writer_.write_event(Event::PI(BytesPI::new(text))).unwrap();
        self
    }
}


impl Responder for XmlDocument {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        let result = Bytes::from(self.writer_.into_inner().into_inner());
        HttpResponse::Ok()
            .content_type("application/xml")
            .body(result)
    }
    
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/test.xml")]
async fn test() -> impl Responder {
    let mut doc = XmlDocument::new();
    {
        doc.pi("processing instruction");
        let mut root = doc.tag("root");
        {
            let mut child = root.tag_with_attrs("child", vec![Attribute::from(("name", "value"))]);
            child.text("This is a child element with attributes.");
        }
    }
    doc
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    HttpServer::new(|| {
        App::new()
            .service(hello)
            .service(test)
    })
    .bind(("::", 8080))?
    .run()
    .await
}