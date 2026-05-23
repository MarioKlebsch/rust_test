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

// # XML Document Building with XmlDocument and XmlTag
//
// This module provides a fluent, RAII-based API for building XML documents in memory.
//
// ## Quick Start
//
// ```ignore
// use quick_xml::events::attributes::Attribute;
//
// let mut doc = XmlDocument::new();
// let mut root = doc.tag("root");
// root.text("Hello, World!");
// ```
//
// ## Overview
//
// - **`XmlDocument`** - The root container for XML documents. Creates an in-memory XML document
//   with a standard XML declaration and manages the underlying writer.
//
// - **`XmlTag`** - Represents an open XML element. Borrows from the parent container's writer,
//   allowing nested elements. Automatically closes the tag when dropped (RAII pattern).
//
// - **`XmlContainer`** - A trait implemented by both `XmlDocument` and `XmlTag` that provides
//   methods for adding content and creating nested elements.
//
// ## Usage Patterns
//
// ### Creating Elements
//
// Use `.tag()` to create a new element:
//
// ```ignore
// let mut doc = XmlDocument::new();
// let mut root = doc.tag("root");
// let mut child = root.tag("child");
// child.text("Content");
// // child closes here when dropped
// // root closes here when dropped
// ```
//
// ### Adding Attributes
//
// Use `.tag_with_attrs()` with a vector of `Attribute`:
//
// ```ignore
// let mut elem = root.tag_with_attrs("element", vec![
//     Attribute::from(("id", "123")),
//     Attribute::from(("class", "item")),
// ]);
// ```
//
// ### Adding Content
//
// Multiple content types are supported with method chaining:
//
// ```ignore
// root
//     .text("Regular text content")
//     .cdata("Raw <content> without escaping")
//     .pi("xml-stylesheet href='style.css'")
//     .comment("This is a note")
//     .text("More text");
// ```
//
// ### HTTP Responses
//
// `XmlDocument` implements `Responder` for actix-web, allowing direct use in route handlers:
//
// ```ignore
// #[get("/data.xml")]
// async fn get_xml() -> impl Responder {
//     let mut doc = XmlDocument::new();
//     let mut root = doc.tag("data");
//     root.text("Content");
//     doc  // Automatically serialized to HTTP response
// }
// ```
//
// ## Automatic Cleanup
//
// Tags are automatically closed via the `Drop` trait. Scoping ensures proper XML structure:
//
// ```ignore
// let mut root = doc.tag("root");
// {
//     let mut child = root.tag("child");
//     child.text("Content");
//     // child is closed here
// }
// // root is closed here
// ```
//
// ## Method Chaining
//
// Most trait methods return `&mut Self`, enabling fluent API chains:
//
// ```ignore
// root
//     .text("Hello")
//     .text(" World")
//     .comment("Done");
// ```
//
// ## Character Escaping
//
// - `.text()` automatically escapes special XML characters (`<`, `>`, `&`)
// - `.cdata()` embeds content as-is without escaping
// - Attributes are automatically escaped

/// A trait for building XML documents with a fluent builder interface.
///
/// `XmlContainer` provides methods to construct XML elements, add content, and manage
/// the underlying writer. It supports creating nested tags, adding text, processing
/// instructions, CDATA sections, and comments.
///
/// # Examples
///
/// ```ignore
/// let mut doc = XmlDocument::new();
/// let mut root = doc.tag("root");
/// root.text("Hello, World!");
/// ```
///
/// # Generic Parameters
///
/// * `W` - The underlying writer type that implements `std::io::Write`.
#[allow(dead_code)]
trait XmlContainer<W: std::io::Write> {
    /// Returns a mutable reference to the underlying XML writer.
    fn writer(&mut self) -> &mut Writer<W>;

    /// Creates a new XML element (tag) with the given name.
    ///
    /// The tag will be automatically closed when the returned `XmlTag` is dropped,
    /// making it safe to use in scoped contexts.
    ///
    /// # Arguments
    ///
    /// * `tag_name` - The name of the XML element to create.
    ///
    /// # Returns
    ///
    /// An `XmlTag` that represents the opened element and can be used to add content or nested elements.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut root = doc.tag("root");
    /// // root is automatically closed when dropped
    /// ```
    fn tag<'a>(&'a mut self, tag_name: &str) -> XmlTag<'a, W>{
        XmlTag::new(self.writer(), tag_name)
    }

    /// Creates a new XML element (tag) with the given name and attributes.
    ///
    /// Similar to `tag()`, but allows specifying attributes on the element.
    /// The tag will be automatically closed when the returned `XmlTag` is dropped.
    ///
    /// # Arguments
    ///
    /// * `tag_name` - The name of the XML element to create.
    /// * `attributes` - An iterable collection of attributes to apply to the element.
    ///
    /// # Returns
    ///
    /// An `XmlTag` that represents the opened element with the specified attributes.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut child = root.tag_with_attrs("child", vec![Attribute::from(("id", "42"))]);
    /// ```
    fn tag_with_attrs<'a, I>(&'a mut self, tag_name: &str, attributes: I) -> XmlTag<'a, W>
    where
        I: IntoIterator<Item = Attribute<'a>> {
        XmlTag::new_with_attrs(self.writer(), tag_name, attributes)
    }

    /// Adds text content to the current element.
    ///
    /// Returns a mutable reference to self for method chaining.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to add. Special XML characters are automatically escaped.
    ///
    /// # Returns
    ///
    /// A mutable reference to self, enabling fluent chaining.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// root.text("Hello, World!");
    /// ```
    fn text(&mut self, text: &str) -> &mut Self{
        self.writer().write_event(Event::Text(BytesText::new(text))).unwrap();
        self
    }

    /// Adds an XML processing instruction (PI) to the current element.
    ///
    /// Processing instructions are used for non-XML information within the document.
    /// Returns a mutable reference to self for method chaining.
    ///
    /// # Arguments
    ///
    /// * `text` - The content of the processing instruction.
    ///
    /// # Returns
    ///
    /// A mutable reference to self, enabling fluent chaining.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// doc.pi("xml-stylesheet href='style.css' type='text/css'");
    /// ```
    fn pi(&mut self, text: &str) -> &mut Self{
        self.writer().write_event(Event::PI(BytesPI::new(text))).unwrap();
        self
    }

    /// Adds a CDATA (Character Data) section to the current element.
    ///
    /// CDATA sections allow including text with special characters without escaping.
    /// Returns a mutable reference to self for method chaining.
    ///
    /// # Arguments
    ///
    /// * `text` - The content of the CDATA section.
    ///
    /// # Returns
    ///
    /// A mutable reference to self, enabling fluent chaining.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// root.cdata("This contains <special> characters & symbols");
    /// ```
    fn cdata(&mut self, text: &str) -> &mut Self{
        self.writer().write_event(Event::CData(BytesCData::new(text))).unwrap();
        self
    }

    /// Adds an XML comment to the current element.
    ///
    /// Comments are not processed and are for documentation purposes.
    /// Returns a mutable reference to self for method chaining.
    ///
    /// # Arguments
    ///
    /// * `text` - The comment text. Should not contain "--" sequences.
    ///
    /// # Returns
    ///
    /// A mutable reference to self, enabling fluent chaining.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// doc.comment("This is a comment");
    /// ```
    fn comment(&mut self, text: &str) -> &mut Self{
        self.writer().write_event(Event::Comment(BytesText::new(text))).unwrap();
        self
    }
}

/// Represents an open XML element within a document.
///
/// `XmlTag` is a builder type that allows fluent construction of XML content within a tag.
/// It holds a reference to the underlying writer and the tag name, and automatically closes
/// the tag when dropped via the `Drop` trait implementation.
///
/// # Generic Parameters
///
/// * `'a` - The lifetime of the borrowed writer.
/// * `W` - The underlying writer type that implements `std::io::Write`.
///
/// # Automatic Cleanup
///
/// The tag is automatically closed (an end tag is written) when the `XmlTag` value is dropped.
/// This ensures proper XML structure even in the presence of early returns or errors.
///
/// # Examples
///
/// ```ignore
/// let mut doc = XmlDocument::new();
/// let mut root = doc.tag("root");
/// root.text("Hello, World!");
/// // root is automatically closed here when dropped
/// ```
struct XmlTag<'a, W: std::io::Write> {
    writer_: &'a mut quick_xml::Writer<W>,
    tag_name: String,
}

impl <'a, W: std::io::Write> XmlTag<'a, W> {
    /// Creates a new XML element with the given name.
    ///
    /// This method writes the opening tag to the writer and returns an `XmlTag` instance
    /// that can be used to add content. The closing tag is automatically written when
    /// the returned `XmlTag` is dropped.
    ///
    /// # Arguments
    ///
    /// * `parent` - A mutable reference to the underlying XML writer.
    /// * `tag_name` - The name of the XML element to create.
    ///
    /// # Returns
    ///
    /// An `XmlTag` instance representing the newly opened element.
    ///
    /// # Panics
    ///
    /// Panics if the write operation fails (unwraps the result).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut writer = Writer::new(Cursor::new(Vec::new()));
    /// let tag = XmlTag::new(&mut writer, "element");
    /// ```
    pub fn new(parent: &'a mut quick_xml::Writer<W>, tag_name: &str) -> Self {
        parent.write_event(Event::Start(BytesStart::new(tag_name))).unwrap();
        XmlTag { writer_: parent, tag_name: tag_name.to_string() }
    }

    /// Creates a new XML element with the given name and attributes.
    ///
    /// Similar to `new()`, but allows specifying attributes on the opening tag.
    /// The attributes are applied to the opening tag, and the closing tag is
    /// automatically written when the returned `XmlTag` is dropped.
    ///
    /// # Arguments
    ///
    /// * `parent` - A mutable reference to the underlying XML writer.
    /// * `tag_name` - The name of the XML element to create.
    /// * `attributes` - An iterable collection of attributes to apply to the element.
    ///                  Each item must implement or be convertible to `Attribute<'b>`.
    ///
    /// # Returns
    ///
    /// An `XmlTag` instance representing the newly opened element with attributes.
    ///
    /// # Panics
    ///
    /// Panics if the write operation fails (unwraps the result).
    ///
    /// # Generic Parameters
    ///
    /// * `'b` - The lifetime of the attributes.
    /// * `I` - The iterator type of the attributes.
    /// * `A` - The attribute type, which must be convertible to `Attribute<'b>`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut writer = Writer::new(Cursor::new(Vec::new()));
    /// let attrs = vec![Attribute::from(("id", "42")), Attribute::from(("class", "item"))];
    /// let tag = XmlTag::new_with_attrs(&mut writer, "div", attrs);
    /// ```
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

/// Automatically closes the XML tag when it goes out of scope.
///
/// The `Drop` implementation ensures that the closing tag is written to the underlying
/// writer when the `XmlTag` is dropped. This provides RAII-style resource management
/// for XML elements, ensuring proper structure even if a function returns early or panics.
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

/// Represents an in-memory XML document that can be built and serialized.
///
/// `XmlDocument` is the root container for building XML documents. It creates a complete
/// XML document with an XML declaration (`<?xml version="1.0" encoding="UTF-8"?>`) and
/// manages the underlying writer that accumulates the XML content.
///
/// The document writes to an in-memory buffer (using `Cursor<Vec<u8>>`), allowing the
/// complete XML to be retrieved as bytes and sent as HTTP responses or saved to files.
///
/// # Implements
///
/// * `XmlContainer<Cursor<Vec<u8>>>` - Provides methods to add elements and content to the document.
/// * `Responder` - Compatible with actix-web for direct HTTP responses.
///
/// # Examples
///
/// ```ignore
/// let mut doc = XmlDocument::new();
/// let mut root = doc.tag("root");
/// root.text("Hello, World!");
/// // doc is ready to be sent as an HTTP response
/// ```
///
/// # Method Chaining
///
/// Many methods return `&mut Self`, enabling fluent builder-style API chains.
///
/// # Encoding
///
/// Documents are created with UTF-8 encoding and standalone declaration set to "yes".
struct XmlDocument {
    writer_: Writer<Cursor<Vec<u8>>>,
}

impl XmlDocument {
    /// Creates a new XML document with standard XML declaration.
    ///
    /// Initializes an empty XML document and writes the XML declaration header:
    /// `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>`.
    ///
    /// The document is ready to have elements, text, and other content added via the
    /// `XmlContainer` trait methods.
    ///
    /// # Returns
    ///
    /// A new `XmlDocument` instance with an initialized writer and XML declaration.
    ///
    /// # Panics
    ///
    /// Panics if the XML declaration cannot be written (unwraps the result).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let doc = XmlDocument::new();
    /// // Document is ready with XML declaration already written
    /// ```
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

    /// Adds a DOCTYPE declaration to the document.
    ///
    /// Inserts a DOCTYPE declaration after the XML declaration. This is optional and
    /// typically used to specify the document type or external DTD references.
    ///
    /// Returns a mutable reference to self for method chaining.
    ///
    /// # Arguments
    ///
    /// * `text` - The DOCTYPE declaration text. For example: `"html"` or `"html PUBLIC \"-//W3C//DTD HTML 4.01//EN\""`.
    ///
    /// # Returns
    ///
    /// A mutable reference to self, enabling fluent chaining.
    ///
    /// # Panics
    ///
    /// Panics if the write operation fails (unwraps the result).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut doc = XmlDocument::new();
    /// doc.doctype("html");
    /// let mut root = doc.tag("html");
    /// ```
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


/// Converts `XmlDocument` into an HTTP response compatible with actix-web.
///
/// This implementation allows an `XmlDocument` to be returned directly from actix-web
/// route handlers, automatically serializing the XML content into the HTTP response body
/// with the correct `application/xml` content type.
///
/// The implementation extracts the accumulated XML bytes from the internal writer
/// and returns them as the response body.
///
/// # Content Type
///
/// The response is automatically set to `application/xml`, ensuring proper MIME type handling.
///
/// # Panics
///
/// Panics if extracting bytes from the writer fails (unwraps the result).
///
/// # Examples
///
/// ```ignore
/// #[get("/data.xml")]
/// async fn get_xml() -> impl Responder {
///     let mut doc = XmlDocument::new();
///     let mut root = doc.tag("root");
///     root.text("Data");
///     doc  // Automatically converted to HTTP response
/// }
/// ```
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