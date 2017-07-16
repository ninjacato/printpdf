//! Embedding fonts in 2D for Pdf
extern crate lopdf;
extern crate freetype as ft;

use *;
use std::collections::HashMap;

/// The font
#[derive(Debug, Clone)]
pub struct Font {
    /// Font data
    pub(crate) font_bytes: Vec<u8>,
    /// Font name, for adding as a resource on the document
    pub(crate) face_name: String,
}

impl Font {
    pub fn new<R>(mut font_stream: R)
    -> std::result::Result<Self, Error> where R: ::std::io::Read
    {
        // read font from stream and parse font metrics
        let mut buf = Vec::<u8>::new();
        font_stream.read_to_end(&mut buf)?;

        let face_name = {
            let library = ft::Library::init().unwrap();
            let face = library.new_memory_face(&buf, 0).unwrap();
            face.postscript_name().expect("Could not read font name!")
        };

        Ok(Self {
            font_bytes: buf,
            face_name: face_name,
        })
    }

    /// Takes the font and adds it to the document and consumes the font
    pub(crate) fn into_obj_with_document(self, doc: &mut lopdf::Document)
    ->lopdf::Dictionary
    {
        use lopdf::Object::*;
        use lopdf::Object;
        use lopdf::{Stream as LoStream, Dictionary as LoDictionary};
        use lopdf::StringFormat;
        use std::collections::BTreeMap;
        use std::iter::FromIterator;

        let face_name = self.face_name.clone();

        let font_buf_ref: Box<[u8]> = self.font_bytes.into_boxed_slice();
        let library = ft::Library::init().unwrap();
        let face = library.new_memory_face(&*font_buf_ref, 0).unwrap();

        // Extract basic font information
        // TODO: return specific error when returning
        let face_metrics = face.size_metrics().expect("Could not read font metrics!");

        let font_stream = LoStream::new(
            LoDictionary::from_iter(vec![
                ("Length1", Integer(font_buf_ref.len() as i64)),
                ("Subtype", Name("CIDFontType0C".into())),
                ]),
            font_buf_ref.to_vec())
        .with_compression(false); /* important! font stream must not be compressed! */

        // Begin setting required font attributes
        let mut font_vec: Vec<(std::string::String, Object)> = vec![
            ("Type".into(), Name("Font".into())),
            ("Subtype".into(), Name("Type0".into())),
            ("BaseFont".into(), Name(face_name.clone().into_bytes())),
            ("Encoding".into(), Name("Identity-H".into())),
            /* Missing DescendantFonts and ToUnicode */
        ];

        let mut font_descriptor_vec: Vec<(std::string::String, Object)> = vec![
            ("Type".into(), Name("FontDescriptor".into())),
            ("FontName".into(), Name(face_name.clone().into_bytes())),
            ("Ascent".into(), Integer(face_metrics.ascender)),
            ("Descent".into(), Integer(face_metrics.descender)),
            ("CapHeight".into(), Integer(face_metrics.ascender)),
            ("ItalicAngle".into(), Integer(0)),
            ("Flags".into(), Integer(32)),
            ("StemV".into(), Integer(80)),
        ];
        // End setting required font arguments

        let mut max_height = 0;             // Maximum height of the font
        let mut total_width = 0;            // Total width of all characters
        let mut widths = Vec::<Object>::new();             // Widths of the individual characters
        let mut cmap = BTreeMap::<u32, (u32, u32)>::new(); // Glyph IDs - (Unicode IDs - character width)
        cmap.insert(0, (0, 1000));

        // face.set_pixel_sizes(1000, 0).unwrap(); // simulate points

        for unicode in 0x0000..0xffff {
            let glyph_id = face.get_char_index(unicode);
            if glyph_id != 0 {
                // this should not fail - if we can get the glyph id, we can get the glyph itself
                if face.load_glyph(glyph_id, ft::face::NO_SCALE).is_ok() {
                    
                    let glyph_slot = face.glyph();
                    let glyph_metrics = glyph_slot.metrics();

                    let w = glyph_metrics.width;
                    let h = glyph_metrics.height;

                    if h > max_height{
                        max_height = h;
                    };

                    total_width += w;
                    cmap.insert(glyph_id, (unicode as u32, w as u32));
                }
            }
        }

        // Maps the character index to a unicode value
        // Add this to the "ToUnicode" dictionary
        // To explain this structure: Glyph IDs have to be in segments where the first byte of the
        // first and last element have to be the same. A range from 0x1000 - 0x10FF is valid
        // but a range from 0x1000 - 0x12FF is not (0x10 != 0x12)
        // Plus, the maximum number of Glyph-IDs in one range is 100
        // Since the glyph IDs are sequential, all we really have to do is to enumerate the vector
        // and create buckets of 100 / rest to 256 if needed
        let mut cid_to_unicode_map = format!(include_str!("../../../../templates/gid_to_unicode_beg.txt"), 
                                             face_name.clone());

        let mut cur_block_id: u32 = 0;          // ID of the block, to be used it {} beginbfchar
        let mut cur_first_bit: u16 = 0_u16;     // current first bit of the glyph id (0x10 or 0x12) for example
        let mut last_block_begin: u32 = 0;      // glyph ID of the start of the current block,
                                                // to satisfy the "less than 100 entries per block" rule

        for (glyph_id, unicode_width_tuple) in cmap.iter() {

            if (*glyph_id >> 8) as u16 != cur_first_bit || *glyph_id > last_block_begin + 100 {
                cid_to_unicode_map.push_str("endbfchar\r\n");
                cur_block_id += 1;
                last_block_begin = *glyph_id;
                cur_first_bit = (*glyph_id >> 8) as u16;
                cid_to_unicode_map.push_str(format!("{} beginbfchar\r\n", cur_block_id).as_str());
            }

            let unicode = unicode_width_tuple.0;
            let width = unicode_width_tuple.1;
            cid_to_unicode_map.push_str(format!("<{:04x}> <{:04x}>\n", glyph_id, unicode).as_str());
            widths.push(Integer(width as i64));
        };

        if cmap.len() % 256 != 0 || cmap.len() % 100 != 0 {
            cid_to_unicode_map.push_str("endbfchar\r\n");
        }

        cid_to_unicode_map.push_str(include_str!("../../../../templates/gid_to_unicode_end.txt"));
        let cid_to_unicode_map_stream = LoStream::new(LoDictionary::new(), cid_to_unicode_map.as_bytes().to_vec());
        let cid_to_unicode_map_stream_id = doc.add_object(cid_to_unicode_map_stream);

        let mut desc_fonts = LoDictionary::from_iter(vec![
            ("Type", Name("Font".into())),
            ("Subtype", Name("CIDFontType0".into())),
            ("BaseFont", Name(face_name.clone().into())),
            ("W",  Array(vec![Integer(0), Array(widths)])),
            /* W2 for vertical writing? */
            ("CIDSystemInfo", Dictionary(LoDictionary::from_iter(vec![
                    ("Registry", String("Adobe".into(), StringFormat::Literal)),
                    ("Ordering", String("Identity".into(), StringFormat::Literal)),
                    ("Supplement", Integer(0)),
            ]))),
            /* CIDToGIDMap ??? */
        ]);

        let font_bbox = vec![ Integer(0), Integer(max_height as i64), Integer(total_width as i64), Integer(max_height as i64) ];
        font_descriptor_vec.push(("FontBBox".into(), Array(font_bbox)));
        font_descriptor_vec.push(("FontFile3".into(), Reference(doc.add_object(font_stream))));
        let font_descriptor_vec_id = doc.add_object(LoDictionary::from_iter(font_descriptor_vec));
        
        desc_fonts.set("FontDescriptor", Reference(font_descriptor_vec_id));

        font_vec.push(("DescendantFonts".into(), Array(vec![Dictionary(desc_fonts)])));
        font_vec.push(("ToUnicode".into(), Reference(cid_to_unicode_map_stream_id)));
               
        lopdf::Dictionary::from_iter(font_vec)
    }
}

impl PartialEq for Font {
    /// Two fonts are equal if their names are equal, the contents aren't checked
    fn eq(&self, other: &Font) -> bool {
        self.face_name == other.face_name
    }
}

/// Indexed reference to a font that was added to the document
/// This is a "reference by postscript name"
#[derive(Debug, Hash, Eq, Clone, PartialEq)]
pub struct IndirectFontRef {
    /// Name of the font
    pub(crate) name: String,
}

/// Direct reference (wrapper for lopdf::Object::Reference) 
/// for increased type safety
#[derive(Debug, Clone)]
pub struct DirectFontRef {
    /// Reference to the content in the document stream
    pub(crate) inner_obj: lopdf::ObjectId,
    /// Actual font data 
    pub(crate) data: Font,
}

impl IndirectFontRef {
    /// Creates a new IndirectFontRef from an index
    pub fn new(index: usize)
    -> Self 
    {
        Self {
            name: format!("F{}", index),
        }
    }
}

/// Font list for tracking fonts within a single PDF document
#[derive(Debug)]
pub struct FontList {
    fonts: HashMap<IndirectFontRef, DirectFontRef>,
}

impl FontList {
    
    /// Creates a new FontList
    pub fn new()
    -> Self
    {
        Self {
            fonts: HashMap::new(),
        }
    }

    /// Adds a font to the FontList
    pub fn add_font(&mut self, font_ref: IndirectFontRef, font: DirectFontRef)
    -> IndirectFontRef
    {
        self.fonts.insert(font_ref.clone(), font);
        font_ref
    }

    /// Turns an indirect font reference into a direct one 
    /// (Warning): clones the direct font reference
    #[inline]
    pub fn get_font(&self, font: &IndirectFontRef)
    -> Option<DirectFontRef>
    {
        let font_ref = self.fonts.get(font);
        if let Some(r) = font_ref {
            Some(r.clone())
        } else {
            None
        }
    }

    /// Returns the number of fonts currenly in use
    #[inline]
    pub fn len(&self)
    -> usize 
    {
        self.fonts.len()
    }
}

impl Into<lopdf::Dictionary> for FontList {
    fn into(self)
    -> lopdf::Dictionary
    {
        let mut font_dict = lopdf::Dictionary::new();
        
        for (indirect_ref, direct_ref) in self.fonts.into_iter() {
            font_dict.set(indirect_ref.name, lopdf::Object::Reference(direct_ref.inner_obj));
        }

        return font_dict;
    }
}