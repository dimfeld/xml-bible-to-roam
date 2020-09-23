use itertools::join;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::Serialize;
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::mem;
use structopt::StructOpt;

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Could not open bible XML file: {}", source))]
    OpenXML { source: quick_xml::Error },

    #[snafu(display("Unable to parse chapter {}", chapter))]
    BadChapter { chapter: String },

    #[snafu(display("Error parsing XML: {}", source))]
    ParseError { source: quick_xml::Error },

    #[snafu(display("Writing output: {}", source))]
    WriteJSON { source: serde_json::Error },

    #[snafu(display("Output Error: {}", source))]
    IOError { source: std::io::Error },
}

#[derive(Debug, StructOpt)]
struct Config {
    #[structopt(short = "c")]
    chapters: Vec<String>,

    #[structopt(short = "f", long = "file", default_value = "ESV.xml")]
    path: std::path::PathBuf,
}

#[derive(Serialize, Debug)]
struct RoamDocument {
    title: String,
    children: Vec<RoamBlock>,
}

#[derive(Serialize, Debug)]
struct RoamBlock {
    string: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    heading: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<RoamBlock>>,
}

fn get_name<'a>(e: &'a quick_xml::events::BytesStart) -> String {
    e.attributes()
        .map(|a| a.unwrap())
        .find(|a| a.key == b"n")
        .map(|a| String::from_utf8_lossy(&a.unescaped_value().unwrap()).to_string())
        .unwrap()
}

struct BookAndChapter {
    book: String,
    chapter: usize,
}

fn parse_chapter(s: String) -> Result<BookAndChapter, Error> {
    let mut tokens = s.split_whitespace();
    let chapter_as_string = tokens.next_back().ok_or_else(|| Error::BadChapter {
        chapter: String::from(&s),
    })?;
    let book = join(tokens, " ");
    let chapter: usize = chapter_as_string.parse().map_err(|_| Error::BadChapter {
        chapter: String::from(&s),
    })?;

    Ok(BookAndChapter { book, chapter })
}

fn main() -> Result<(), Error> {
    let config = Config::from_args();
    let mut reader = Reader::from_file(config.path).context(OpenXML {})?;
    let mut buf = Vec::new();

    let expected_chapters = config
        .chapters
        .into_iter()
        .map(parse_chapter)
        .collect::<Result<Vec<BookAndChapter>, Error>>()?;

    let num_expected_chapters = expected_chapters.len();

    let all_expected_books: HashMap<String, Vec<usize>> =
        expected_chapters
            .into_iter()
            .fold(HashMap::new(), |mut acc, cb| {
                acc.entry(cb.book).or_insert_with(Vec::new).push(cb.chapter);
                acc
            });

    let mut in_book: Option<(String, &[usize])> = None;
    let mut in_chapter: Option<usize> = None;
    let mut in_verse: Option<String> = None;

    let mut verses = Vec::new();
    let mut finished_chapters: Vec<(BookAndChapter, Vec<String>)> = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match (e.name(), &in_book, in_chapter) {
                    (b"b", &None, _) => {
                        // Book
                        let book = get_name(e);
                        in_book = all_expected_books
                            .get(&book)
                            .map(|chapters| (book.to_string(), chapters.as_slice()));
                    }
                    (b"c", Some((_, chapters)), _) => {
                        let chapter_num = get_name(e).parse::<usize>().unwrap();
                        if chapters.iter().any(|c| *c == chapter_num) {
                            in_chapter = Some(chapter_num);
                        } else {
                            in_chapter = None;
                        }
                    }
                    (b"v", Some(_), Some(_)) => {
                        in_verse = Some(get_name(e).to_string());
                    }
                    _ => (),
                }
            }
            Ok(Event::End(ref e)) => match (e.name(), &in_book, in_chapter) {
                (b"b", _, _) => {
                    in_book = None;
                    in_chapter = None;
                }
                (b"c", Some((book, _)), Some(chapter)) => {
                    // Finished the chapter we're looking for.
                    let chapter_verses = mem::replace(&mut verses, Vec::new());
                    finished_chapters.push((
                        BookAndChapter {
                            book: book.to_string(),
                            chapter,
                        },
                        chapter_verses,
                    ));

                    in_chapter = None;

                    if finished_chapters.len() == num_expected_chapters {
                        // All done!
                        break;
                    }
                }
                (b"v", Some(_), Some(_)) => {
                    in_verse = None;
                }
                _ => (),
            },
            Ok(Event::Text(ref t)) => match (&in_book, in_chapter, &in_verse) {
                (Some(_), Some(_), Some(verse)) => {
                    let value = t.unescape_and_decode(&reader).unwrap();
                    verses.push(format!("{}. {}", verse, value));
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::ParseError { source: e }),
            _ => (),
        }
    }

    let docs = finished_chapters
        .into_iter()
        .map(|(bc, verses)| {
            let verse_blocks = verses
                .into_iter()
                .map(|v| RoamBlock {
                    string: v,
                    heading: None,
                    children: None,
                })
                .collect::<Vec<_>>();

            RoamDocument {
                title: format!("{} {}", bc.book, bc.chapter),
                children: vec![
                    RoamBlock {
                        string: format!("Bible Book:: [[{}]]", bc.book),
                        heading: None,
                        children: None,
                    },
                    RoamBlock {
                        string: format!("[[{} {}]]", bc.book, bc.chapter),
                        heading: None,
                        children: Some(verse_blocks),
                    },
                ],
            }
        })
        .collect::<Vec<_>>();

    serde_json::to_writer(std::io::stdout(), &docs).context(WriteJSON)?;

    Ok(())
}
