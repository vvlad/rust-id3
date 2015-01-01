use std::io::{SeekEnd, IoResult};

static TAG: &'static [u8] = b"TAG";
pub static TAG_OFFSET: i64 = 128;
static TITLE_OFFSET: i64 = 125;
static TITLE_LEN: uint = 30;
static ARTIST_OFFSET: i64 = 95;
static ARTIST_LEN: uint = 30;
static ALBUM_OFFSET: i64 = 65;
static ALBUM_LEN: uint = 30;
static YEAR_OFFSET: i64 = 35;
static YEAR_LEN: uint = 4;
static COMMENT_OFFSET: i64 = 31;
static COMMENT_LEN: uint = 30;
static TRACK_OFFSET: i64 = 3;
static GENRE_OFFSET: i64 = 1;

static TAGPLUS: &'static [u8] = b"TAG+";
pub static TAGPLUS_OFFSET: i64 = 355;
static XTITLE_OFFSET: i64 = 351;
static XTITLE_LEN: uint = 60;
static XARTIST_OFFSET: i64 = 291;
static XARTIST_LEN: uint = 60;
static XALBUM_OFFSET: i64 = 231;
static XALBUM_LEN: uint = 60;
static SPEED_OFFSET: i64 = 171;
static GENRE_STR_OFFSET: i64 = 170;
static GENRE_STR_LEN: uint = 30;
static START_TIME_OFFSET: i64 = 140;
static START_TIME_LEN: uint = 6;
static END_TIME_OFFSET: i64 = 134;
static END_TIME_LEN: uint = 6;

/// A structure containing ID3v1 metadata.
pub struct ID3v1 {
    /// The full title (ID3v1 + extension if present).
    pub title: Option<String>,
    /// The full artist (ID3v1 + extension if present).
    pub artist: Option<String>,
    /// The full album (ID3v1 + extension if present).
    pub album: Option<String>,
    /// A 4-digit string, if we are lucky
    pub year: Option<String>,
    /// A free-form comment.
    pub comment: Option<String>,
    /// Number of the track, 0 if not set. ID3v1.1 data.
    pub track: Option<u8>,
    /// The genre mapping is standardized up to 79, some extensions exist.
    /// http://eyed3.nicfit.net/plugins/genres_plugin.html
    pub genre: Option<u8>,
    /// 1 (slow), 2, 3, 4 (fast) or 0 (not set). ID3v1 extended data.
    pub speed: Option<u8>,
    /// Free-form genre string. ID3v1 extended data.
    pub genre_str: Option<String>,
    /// The real start of the track, mmm:ss. ID3v1 extended data.
    pub start_time: Option<String>,
    /// The real end of the track, mmm:ss. ID3v1 extended data.
    pub end_time: Option<String>,
}

impl ID3v1 {
    /// Creates a new ID3v1 tag with no information.
    pub fn new() -> ID3v1 {
        ID3v1 {
            title: None, artist: None, album: None, year: None, comment: None, track: None,
            genre: None, speed: None, genre_str: None, start_time: None, end_time: None
        }
    }
}

/// ID3v1 tag reading helpers.
trait ID3v1Helpers {
    /// Read `n` bytes starting at an offset from the end.
    fn read_from_end(&mut self, n:uint, offset:i64) -> IoResult<Vec<u8>>;

    /// Read a null-terminated ISO-8859-1 string of size at most `n`, at an offset from the end.
    fn read_str(&mut self, n: uint, offset: i64) -> IoResult<String>;
}

impl<R: Reader + Seek> ID3v1Helpers for R {
    #[inline]
    fn read_from_end(&mut self, n: uint, offset:i64) -> IoResult<Vec<u8>> {
        try!(self.seek(-offset, SeekEnd));
        self.read_exact(n)
    }

    #[inline]
    fn read_str(&mut self, n: uint, offset: i64) -> IoResult<String> {
        self.read_from_end(n, offset).map(|vec| extract_nz_88591(vec))
    }
}

/// Checks for the existence of the bytes denoting an ID3v1 metadata block tag.
#[inline]
pub fn probe_tag<R: Reader + Seek>(reader: &mut R) -> IoResult<bool> {
    let tag = try!(reader.read_from_end(TAG.len(), TAG_OFFSET));
    Ok(TAG == tag.as_slice())
}

/// Checks for the existence of the bytes denoting an ID3v1 extended metadata tag.
#[inline]
pub fn probe_xtag<R: Reader + Seek>(reader: &mut R) -> IoResult<bool> {
    let xtag = try!(reader.read_from_end(TAGPLUS.len(), TAGPLUS_OFFSET));
    Ok(TAGPLUS == xtag.as_slice())
}

pub fn read<R: Reader + Seek>(reader: &mut R) -> IoResult<ID3v1> {
    macro_rules! maybe_read {
        ($prop:expr, $len:ident, $offset:ident) => {
            {
                let mut string = $prop.or(Some(String::new())).unwrap();
                string.push_str(try!(reader.read_str($len, $offset)).as_slice());
                $prop = if string.is_empty() {
                    None
                } else {
                    Some(string)
                }
            }
        };
    }

    let mut tag = ID3v1::new();

    // Try to read ID3v1 metadata.
    let has_tag = try!(probe_tag(reader));
    if has_tag {
        maybe_read!(tag.title, TITLE_LEN, TITLE_OFFSET);
        maybe_read!(tag.artist, ARTIST_LEN, ARTIST_OFFSET);
        maybe_read!(tag.album, ALBUM_LEN, ALBUM_OFFSET);
        maybe_read!(tag.year, YEAR_LEN, YEAR_OFFSET);
        maybe_read!(tag.comment, COMMENT_LEN, COMMENT_OFFSET);
        tag.track = {
            try!(reader.seek(-TRACK_OFFSET, SeekEnd));
            // The track value is meaningful only if the guard byte is 0
            let guard_byte = try!(reader.read_byte());
            if guard_byte == 0 {
                Some(try!(reader.read_byte()))
            } else {
                // If the guard value is not 0, then the track value is
                // not known.
                None
            }
        };
        tag.genre = {
            try!(reader.seek(-GENRE_OFFSET, SeekEnd));
            Some(try!(reader.read_byte()))
        };

        // Try to read ID3v1 extended metadata.
        let has_xtag = try!(probe_xtag(reader));
        if has_xtag {
            maybe_read!(tag.title, XTITLE_LEN, XTITLE_OFFSET);
            maybe_read!(tag.artist, XARTIST_LEN, XARTIST_OFFSET);
            maybe_read!(tag.album, XALBUM_LEN, XALBUM_OFFSET);
            tag.speed = {
                try!(reader.seek(-SPEED_OFFSET, SeekEnd));
                Some(try!(reader.read_byte()))
            };
            maybe_read!(tag.genre_str, GENRE_STR_LEN, GENRE_STR_OFFSET);
            maybe_read!(tag.start_time, START_TIME_LEN, START_TIME_OFFSET);
            maybe_read!(tag.end_time, END_TIME_LEN, END_TIME_OFFSET);
        }
    }

    Ok(tag)
}

/// Read a string from a null-terminated ISO-8859-1 byte vector.
///
/// Read the whole vector if there is no null byte.
///
/// This function cannot fail, because UTF-8 is compatible with ISO-8859-1
/// at the code point level.
#[inline]
fn extract_nz_88591(s: Vec<u8>) -> String {
    // This works because the ISO 8859-1 code points match the unicode code
    // points. So,`c as char` will map correctly from ISO to unicode.
    s.into_iter().take_while(|&c| c!=0).map(|c| c as char).collect()
}