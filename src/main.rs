use quick_xml::events::Event;
use quick_xml::Reader;
use serde::Serialize;
use snafu::{ResultExt, Snafu};
use structopt::StructOpt;

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Could not open bible XML file: {}", source))]
    OpenXML { source: quick_xml::Error },

    #[snafu(display("Error parsing XML: {}", source))]
    ParseError { source: quick_xml::Error },

    #[snafu(display("Writing output: {}", source))]
    WriteJSON { source: serde_json::Error },
}

#[derive(Debug, StructOpt)]
struct Config {
    #[structopt(short = "b")]
    book: String,
    #[structopt(short = "c")]
    chapter: u32,

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

fn name_match(e: &quick_xml::events::BytesStart, expected: &str) -> bool {
    e.attributes()
        .map(|a| a.unwrap())
        .find(|a| a.key == b"n")
        .map(|a| String::from_utf8_lossy(&a.unescaped_value().unwrap()) == expected)
        .unwrap_or(false)
}

fn main() -> Result<(), Error> {
    let config = Config::from_args();
    let mut reader = Reader::from_file(config.path).context(OpenXML {})?;
    let mut buf = Vec::new();

    let mut verses = Vec::new();
    let expected_chapter = config.chapter.to_string();
    let mut in_book = false;
    let mut in_chapter = false;
    let mut in_a_verse = false;
    let mut current_verse = String::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match (e.name(), in_book, in_chapter) {
                    (b"b", false, false) => {
                        // Book
                        if name_match(e, &config.book) {
                            in_book = true
                        }
                    }
                    (b"c", true, false) => {
                        if name_match(e, &expected_chapter) {
                            in_chapter = true;
                        }
                    }
                    (b"v", true, true) => {
                        let verse_number =
                            e.attributes().map(|a| a.unwrap()).find(|a| a.key == b"n");
                        if let Some(a) = verse_number {
                            current_verse =
                                String::from_utf8_lossy(&a.unescaped_value().unwrap()).to_string();
                            in_a_verse = true;
                        }
                    }
                    _ => (),
                }
            }
            Ok(Event::End(ref e)) => match (e.name(), in_book, in_chapter) {
                // Finished the chapter we're looking for.
                (b"c", true, true) => break,
                (b"v", true, true) => {
                    in_a_verse = false;
                }
                _ => (),
            },
            Ok(Event::Text(ref t)) => {
                if in_book && in_chapter && in_a_verse {
                    let value = t.unescape_and_decode(&reader).unwrap();
                    verses.push(format!("{}. {}", current_verse, value));
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::ParseError { source: e }),
            _ => (),
        }
    }

    let verse_blocks = verses
        .into_iter()
        .map(|v| RoamBlock {
            string: v,
            heading: None,
            children: None,
        })
        .collect::<Vec<_>>();

    let doc = vec![RoamDocument {
        title: format!("{} {}", config.book, expected_chapter),
        children: vec![
            RoamBlock {
                string: format!("Bible Book:: [[{}]]", config.book),
                heading: None,
                children: None,
            },
            RoamBlock {
                string: format!("[[{} {}]]", config.book, expected_chapter),
                heading: None,
                children: Some(verse_blocks),
            },
        ],
    }];

    serde_json::to_writer(std::io::stdout(), &doc).context(WriteJSON)?;

    Ok(())
}
