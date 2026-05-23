/*
    This is a test project to check out using of quick-xml with actix-web. 

    Copyright (C) 2026  Mario Klebsch, mario@klebsch.de

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use actix_web::web::Bytes;
use quick_xml::events::{BytesDecl, BytesStart, BytesEnd, BytesText, BytesPI, BytesCData, Event};
use quick_xml::events::attributes::Attribute;
use quick_xml::Writer;
use std::io::Cursor;

#[allow(dead_code)]
trait XmlContainer<W: std::io::Write> {
    fn writer(&mut self) -> &mut Writer<W>;

    fn tag<'a>(&'a mut self, tag_name: &str) -> XmlTag<'a, W>{
        XmlTag::new(self.writer(), tag_name)
    }

    fn tag_with_attrs<'a, I>(&'a mut self, tag_name: &str, attributes: I) -> XmlTag<'a, W>
    where
        I: IntoIterator<Item = Attribute<'a>> {
        XmlTag::new_with_attrs(self.writer(), tag_name, attributes)
    }

    fn text(&mut self, text: &str) -> &mut Self{
        self.writer().write_event(Event::Text(BytesText::new(text))).unwrap();
        self
    }

    fn pi(&mut self, text: &str) -> &mut Self{
        self.writer().write_event(Event::PI(BytesPI::new(text))).unwrap();
        self
    }

    fn cdata(&mut self, text: &str) -> &mut Self{
        self.writer().write_event(Event::CData(BytesCData::new(text))).unwrap();
        self
    }

    fn comment(&mut self, text: &str) -> &mut Self{
        self.writer().write_event(Event::Comment(BytesText::new(text))).unwrap();
        self
    }
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

    #[allow(dead_code)]
    fn doctype(&mut self, text: &str) -> &mut Self {
        self.writer_.write_event(Event::DocType(BytesText::new(text))).unwrap();
        self
    }
}

impl XmlContainer<Cursor<Vec<u8>>> for XmlDocument {
    fn writer(&mut self) -> &mut Writer<Cursor<Vec<u8>>> {
        &mut self.writer_
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
        doc.doctype("root");
        doc.pi("processing instruction");
        doc.comment("This is a comment");
        let mut root = doc.tag("root");
        root.cdata("This is a CDATA section");
        let mut child = root.tag_with_attrs("child", vec![Attribute::from(("name", "value<5"))]);
        child.text("This is a child element <with> attributes.");
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