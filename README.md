﻿[![Build Status](https://travis-ci.org/sharazam/printpdf.svg?branch=master)](https://travis-ci.org/sharazam/printpdf)

# printpdf

`printpdf` is a library designed for creating printable PDF documents.

## Getting started

### Writing PDF

There are two types of functions: `add_*` and `use_*`. `add_*`-functions operate on the
document and return a reference to the content that has been added. This is used for 
instantiating objects via references in the document (for example, for reusing a block of 
data - like a font) without copying it (and bloating the file size).

Instancing happens via the `use_*`-functions, which operate on the layer. Meaning, you can only
instantiate blobs / content when you have a reference to the layer. Here are some examples:

#### Simple page

```rust
use printpdf::*;
use std::fs::File;

let (doc, page1, layer1) = PdfDocument::new("PDF_Document_title", 247.0, 210.0, "Layer 1");
let (page2, layer1) = doc.add_page(10.0, 250.0,"Page 2, Layer 1");

doc.save(&mut File::create("test_working.pdf").unwrap()).unwrap();
```

#### Adding graphical shapes

```rust
use printpdf::*;
use std::fs::File;

let (doc, page1, layer1) = PdfDocument::new("PDF_Document_title", 247.0, 210.0, "Layer 1");

let mut current_layer = doc.get_page(page1).get_layer(layer1);

// Quadratic shape. The "false" determines if the next (following) point is a bezier handle (for curves)
let points1 = vec![(Point::new(100.0, 100.0), false),
                   (Point::new(100.0, 200.0), false),
                   (Point::new(300.0, 200.0), false),
                   (Point::new(300.0, 100.0), false)];

// is shape stroked? is shape closed? is shape filled?
let line1 = Line::new(points1, true, true, true);

// set outline and fill
let outline = Outline::new(Color::Rgb(Rgb::new(0.75, 1.0, 0.64, None)));
current_layer.set_outline_color(outline);
current_layer.set_outline_thickness(10); // points

let fill = Fill::new(Color::Cmyk(Cmyk::new(0.0, 0.23, 0.0, 0.0, None)));
current_layer.set_fill(fill);

// add shape to the layer
current_layer.add_shape(line1);
```

#### Adding fonts

```rust
use printpdf::*;
use std::fs::File;

let (doc, page1, layer1) = PdfDocument::new("PDF_Document_title", 247.0, 210.0, "Layer 1");
let text = "Hello World! Unicode test: стуфхfцчшщъыьэюя";
let roboto_font_file = File::open("assets/fonts/RobotoMedium.ttf").unwrap();
let roboto_font = doc.add_font(roboto_font_file).unwrap();

// text, font size, rotation, x from left edge, y from top edge, font
doc.get_page(page1).get_layer(layer1).use_text(text, 48, 0.0, 200.0, 200.0, roboto_font);
```

#### Adding SVG elements

## Goals and Roadmap

The goal of printpdf is to be a general-use PDF library, such as libharu or similar.
PDFs generated by printpdf must always adhere to a PDF standard. However, not all standards 
are supported. See this list:

- [ ] PDF/A-1b:2005
- [ ] PDF/A-1a:2005
- [ ] PDF/A-2:2011
- [ ] PDF/A-2a:2011
- [ ] PDF/A-2b:2011
- [ ] PDF/A-2u:2011
- [ ] PDF/A-3:2012
- [ ] PDF/UA-1
- [ ] PDF/X-1a:2001
- [x] PDF/X-3:2002
- [ ] PDF/X-1a:2003
- [ ] PDF/X-3:2003
- [ ] PDF/X-4:2010
- [ ] PDF/X-4P:2010
- [ ] PDF/X-5G:2010
- [ ] PDF/X-5PG:2010
- [ ] PDF/X-5N:2010
- [ ] PDF/E-1
- [ ] PDF/VT:2010

Over time, there will be more standards supported. Checking a PDF for errors is currently only a stub.

## Testing

Testing should be done in two stages. First, test the individual PDF objects, if the conversion into
a PDF object is done correctly. The second stage is manual inspection of PDF objects via Adobe Preflight.

Put the tests of the first stage in /tests/mod.rs. The second stage tests are better to be handled
inside the plugins' mod.rs file. `printpdf` depends highly on [lopdf](https://github.com/J-F-Liu/lopdf),
so you can either construct your test object against a real type or a debug string of your serialized
type. Either way is fine - you just have to check that the test object is conform to what PDF expects.

## Contibuting

- Fork the project, make you own branch
- If you want to add support for some data type, let's say images or embedded video, create your type
in `/src/types/plugins/[family of your type]/[type].rs`
- The type should implement `IntoPdfObject`, so that it can be added to the document
- Change the `page` and `layer content types to have a convenience function for adding your type
- Document your changes. Add a doc test (how you expect the type to be used) and a unit test 
(if the type is conform to the expected PDF type)
- If you want to change this README, change the lib.rs instead and run `cargo readme > README.md`.
- Create pull request

## Useful links

Here are some esources I found while working on this library

[Official PDF 1.7 reference](http://www.adobe.com/content/dam/Adobe/en/devnet/acrobat/pdfs/pdf_reference_1-7.pdf)

[[GERMAN] How to embed unicode fonts in PDF](http://www.p2501.ch/pdf-howto/typographie/vollzugriff/direkt)

[PDF X/1-a Validator](https://www.pdf-online.com/osa/validate.aspx)

[PDF X/3 technical notes](http://www.pdfxreport.com/lib/exe/fetch.php?media=en:technote_pdfx_checks.pdf)

