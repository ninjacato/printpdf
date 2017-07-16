//! A `PDFDocument` represents the whole content of the file

extern crate lopdf;
extern crate chrono;
extern crate rand;

use *;
use indices::*;
use std::io::{Write, Seek};
use rand::Rng;
use std::rc::Rc;
use std::cell::RefCell;

/// PDF document
#[derive(Debug)]
pub struct PdfDocument {
    /// Pages of the document
    pub(super) pages: Vec<PdfPage>,
    /// Fonts used in this document
    pub fonts: FontList,
    /// ICC profiles used in the document
    pub(super) icc_profiles: IccProfileList,
    /// Inner PDF document
    pub(super) inner_doc: lopdf::Document,
    /// Document ID. Must be changed if the document is loaded / parsed from a file
    pub document_id: std::string::String,
    /// Metadata for this document
    pub metadata: PdfMetadata,
}

/// Marker struct for a document. Used to make the API a bit nicer.
/// It simply calls PdfDocument:: ... functions.
pub struct PdfDocumentReference {
    /// A wrapper for a document, so actions from outside this library
    /// are restricted to functions inside this crate (only functions in `lopdf`
    /// can directly manipulate the document)
    pub(crate) document: Rc<RefCell<PdfDocument>>,
}

impl PdfDocument {

    /// Creates a new PDF document
    #[inline]
    pub fn new<S>(document_title: S, initial_page_width_mm: f64, initial_page_height_mm: f64, 
                  initial_layer_name: S)
    -> (PdfDocumentReference, PdfPageIndex, PdfLayerIndex) where S: Into<String>
    {
        let doc = Self {
            pages: Vec::new(),
            document_id: rand::thread_rng().gen_ascii_chars().take(32).collect(),
            fonts: FontList::new(),
            icc_profiles: IccProfileList::new(),
            inner_doc: lopdf::Document::with_version("1.3"),
            metadata: PdfMetadata::new(document_title, 1, false, PdfConformance::X3_2002_PDF_1_3)
        };

        let doc_ref = Rc::new(RefCell::new(doc));

        let (initial_page, layer_index) = PdfPage::new(
            initial_page_width_mm, 
            initial_page_height_mm, 
            initial_layer_name,
            0);

        { doc_ref.borrow_mut().pages.push(initial_page); }

        (PdfDocumentReference { document: doc_ref }, PdfPageIndex(0), layer_index)
    }

}

impl PdfDocumentReference {

    // ----- BUILDER FUNCTIONS

    /// Changes the title on both the document info dictionary as well as the metadata
    #[inline]
    pub fn with_title<S>(self, new_title: S)
    -> () where S: Into<String>
    {
        self.document.borrow_mut().metadata.document_title = new_title.into();
    }

    /// Set the trapping of the document
    #[inline]
    pub fn with_trapping(self, trapping: bool)
    -> Self 
    {
        self.document.borrow_mut().metadata.trapping = trapping;
        self
    }

    /// Sets the document ID (for comparing two PDF documents for equality)
    #[inline]
    pub fn with_document_id(self, id: String)
    -> Self
    {
        self.document.borrow_mut().metadata.xmp_metadata.document_id = id;
        self
    }

    /// Set the version of the document
    #[inline]
    pub fn with_document_version(self, version: u32)
    -> Self 
    {
        self.document.borrow_mut().metadata.document_version = version;
        self
    }

    /// Changes the conformance of this document. It is recommended to call 
    /// `check_for_errors()` after changing it.
    #[inline]
    pub fn with_conformance(self, conformance: PdfConformance)
    -> Self
    {
        self.document.borrow_mut().metadata.conformance = conformance;
        self
    }

    /// Sets the modification date on the document. Intended to be used when
    /// reading documents that already have a modification date.
    #[inline]
    pub fn with_mod_date(self, mod_date: chrono::DateTime<chrono::Local>)
    -> Self
    {
        self.document.borrow_mut().metadata.modification_date = mod_date;
        self
    }

    // ----- ADD FUNCTIONS

    /// Create a new pdf page and returns the index of the page
    #[inline]
    pub fn add_page<S>(&self, x_mm: f64, y_mm: f64, inital_layer_name: S)
    -> (PdfPageIndex, PdfLayerIndex) where S: Into<String>
    { 
        let mut doc = self.document.borrow_mut();
        let (pdf_page, pdf_layer_index) = PdfPage::new(x_mm, y_mm, inital_layer_name, doc.pages.len());
        doc.pages.push(pdf_page);
        let page_index = PdfPageIndex(doc.pages.len() - 1);
        (page_index, pdf_layer_index)
    }

    /// Add a font from a font stream
    #[inline]
    pub fn add_font<R>(&self, font_stream: R)
    -> ::std::result::Result<IndirectFontRef, Error> where R: ::std::io::Read
    {
        let font = Font::new(font_stream)?; 
        // let name = font.face_name.clone();
        
        let font_ref;

        let possible_ref = {
            let doc = self.document.borrow();
            font_ref = IndirectFontRef::new(font.face_name.clone());
            match doc.fonts.get_font(&font_ref) { Some(f) => Some(f.clone()), None => None }
        };

        if let Some(_) = possible_ref {
            return Ok(font_ref);
        } else {
            let mut doc = self.document.borrow_mut();
            let direct_ref = DirectFontRef { 
                inner_obj: doc.inner_doc.new_object_id(), 
                data: font 
            };

            doc.fonts.add_font(font_ref.clone(), direct_ref);
            return Ok(font_ref);
        }
    }

    // ----- GET FUNCTIONS

    /// Returns the page (for inserting content)
    #[inline]
    pub fn get_page(&self, page: PdfPageIndex)
    -> PdfPageReference
    {
        self.document.borrow_mut().pages.get(page.0).unwrap();
        PdfPageReference { document: Rc::downgrade(&self.document).clone(), page }
    }

    /// Returns a direct reference (object ID) to the font from an 
    /// indirect reference (postscript name)
    #[inline]
    pub fn get_font(&self, font: &IndirectFontRef)
    -> Option<DirectFontRef>
    {
        let doc = self.document.borrow();
        doc.fonts.get_font(font) 
    }

    /// Drops the PDFDocument, returning the inner `lopdf::Document`. 
    /// Document may be only half-written, use only in extreme cases
    #[inline]
    pub unsafe fn get_inner(self)
    -> lopdf::Document
    {
        let doc = Rc::try_unwrap(self.document).unwrap().into_inner();
        doc.inner_doc
    }

    // --- MISC FUNCTIONS

    /// Checks for invalid settings in the document
    pub fn check_for_errors(&self) 
    -> ::std::result::Result<(), Error>
    {
        // todo
        warn!("Checking PDFs for errors is currently not supported!");
        Ok(())
    }

    /// Tries to match the document to the given conformance.
    /// Errors only on an unrecoverable error.
    pub fn repair_errors(&self, conformance: PdfConformance)
    -> ::std::result::Result<(), Error>
    {
        //todo
        warn!("Reparing PDFs is currently not supported!");
        Ok(())
    }

    /// Save PDF Document, writing the contents to the target
    pub fn save<W: Write + Seek>(self, target: &mut W)
    -> ::std::result::Result<(), Error>
    {
        use lopdf::{Dictionary as LoDictionary, 
                    Object as LoObject};
        use lopdf::Object::*;
        use std::iter::FromIterator;
        use lopdf::StringFormat::Literal as Literal;

        // todo: remove unwrap, handle error
        let mut doc = Rc::try_unwrap(self.document).unwrap().into_inner();
        let pages_id = doc.inner_doc.new_object_id();

        // extra pdf infos
        let (xmp_metadata, document_info, icc_profile) = doc.metadata.clone().into_obj();
        let xmp_metadata_id = doc.inner_doc.add_object(xmp_metadata);
        let document_info_id = doc.inner_doc.add_object(document_info);
            
        // add catalog 
        let icc_profile_descr = "Commercial and special offset print acccording to ISO \
                                 12647-2:2004 / Amd 1, paper type 1 or 2 (matte or gloss-coated \
                                 offset paper, 115 g/m2), screen ruling 60/cm";
        let icc_profile_str   = "Coated FOGRA39 (ISO 12647-2:2004)";
        let icc_profile_short = "FOGRA39";

        let mut output_intents = LoDictionary::from_iter(vec![
                          ("S", Name("GTS_PDFX".into())),
                          ("OutputCondition", String(icc_profile_descr.into(), Literal)),
                          ("Type", Name("OutputIntent".into())),
                          ("OutputConditionIdentifier", String(icc_profile_short.into(), Literal)),
                          ("RegistryName", String("http://www.color.org".into(), Literal)),
                          ("Info", String(icc_profile_str.into(), Literal)), 
                        ]);

        if let Some(profile) = icc_profile { 
            let icc_profile: lopdf::Stream = profile.into();
            let icc_profile_id = doc.inner_doc.add_object(lopdf::Object::Stream(icc_profile));
            output_intents.set("DestinationOutputProfile", Reference(icc_profile_id));
        }

        let catalog = LoDictionary::from_iter(vec![
                      ("Type", "Catalog".into()),
                      ("PageLayout", "OneColumn".into()),
                      ("PageMode", "Use0".into()),
                      ("Pages", Reference(pages_id)),
                      ("Metadata", Reference(xmp_metadata_id) ),
                      ("OutputIntents", Array(vec![Dictionary(output_intents)])),
                    ]);

        let mut pages = LoDictionary::from_iter(vec![
                      ("Type", "Pages".into()),
                      ("Count", Integer(doc.pages.len() as i64)),
                      /* Kids and Resources missing */
                      ]);

        // add all pages with contents
        let mut page_ids = Vec::<LoObject>::new();

        for page in doc.pages.into_iter() {
            
            let mut p = LoDictionary::from_iter(vec![
                      ("Type", "Page".into()),
                      ("Rotate", Integer(0)),
                      ("MediaBox", vec![0.into(), 0.into(),
                       page.width_pt.into(), page.heigth_pt.into()].into()),
                      ("TrimBox", vec![0.into(), 0.into(),
                       page.width_pt.into(), page.heigth_pt.into()].into()),
                      ("CropBox", vec![0.into(), 0.into(),
                       page.width_pt.into(), page.heigth_pt.into()].into()),
                      ("Parent", Reference(pages_id)) ]);

            // this will collect the resources needed for rendering this page
            let (resources_page, layer_streams) = page.collect_resources_and_streams(&mut doc.inner_doc);

            if resources_page.len() > 0 {
                let resources_page_id = doc.inner_doc.add_object(lopdf::Object::Dictionary(resources_page));
                p.set("Resources", Reference(resources_page_id));
            }

            // merge layer streams
            let mut layer_streams_merged_vec = Vec::<u8>::new();

            // merge all streams of the individual layers into one big stream
            for mut stream in layer_streams {

                // todo: write begin of pdf layer

                // todo: check if pdf is allowed to have layers
                // if metadata.conformance.is_layering_allowed() { }

                layer_streams_merged_vec.append(&mut stream.content);
                // todo: write end of pdf layer
            }

            let merged_layer_stream = lopdf::Stream::new(lopdf::Dictionary::new(), layer_streams_merged_vec);
            let page_content_id = doc.inner_doc.add_object(merged_layer_stream);
            
            p.set("Contents", Reference(page_content_id));
            page_ids.push(Reference(doc.inner_doc.add_object(p)))
        }

        pages.set::<_, LoObject>("Kids".to_string(), page_ids.into());

        // add all fonts / other resources shared in the whole document
        let fonts_dict: lopdf::Dictionary =  doc.fonts.into_with_document(&mut doc.inner_doc);
        let mut resources_dict: lopdf::Dictionary = lopdf::Dictionary::new();

        if fonts_dict.len() > 0 {
            resources_dict.set("Font", lopdf::Object::Dictionary(fonts_dict));
        }

        if resources_dict.len() > 0 {
            pages.set::<_, LoObject>("Resources".to_string(), resources_dict.into());
        }

        doc.inner_doc.objects.insert(pages_id, Dictionary(pages));

        // save inner document
        let catalog_id = doc.inner_doc.add_object(catalog);
        let instance_id: std::string::String = rand::thread_rng().gen_ascii_chars().take(32).collect();

        doc.inner_doc.trailer.set("Root", Reference(catalog_id));
        doc.inner_doc.trailer.set("Info", Reference(document_info_id));
        doc.inner_doc.trailer.set("ID", Array(vec![
                                            String(doc.document_id.as_bytes().to_vec(), Literal), 
                                            String(instance_id.as_bytes().to_vec(), Literal)
                                        ]));

        // does nothing in debug mode, optimized in release mode
        Self::optimize(&mut doc.inner_doc);
        doc.inner_doc.save_to(target).unwrap();

        Ok(())
    }

    #[cfg(debug_assertions)]
    #[inline]
    fn optimize(_: &mut lopdf::Document) { }

    #[cfg(not(debug_assertions))]
    #[inline]
    fn optimize(doc: &mut lopdf::Document)
    {
        doc.prune_objects();
        doc.delete_zero_length_streams(); 
        doc.compress();
    }
}

/*
impl std::convert::From<lopdf::Doument> for PdfDocument
{
    fn from(doc: lopdf::Doument) -> Self
    {
        
    }
}
*/