# xml-bible-to-roam

A utility to convert a chapter from a Bible in XML format to a JSON file suitable for import into Roam Research. 

The existing Roam public databases for the Bible are only in KJV translation (for licensing reasons), and not being used to King James's English, I find it distracting. There are XML versions of Bibles on the internet (search "opensong xml bible", and so this utility eases the process of importing chapters from those files.

I tend to import chapters as needed, since importing the entire Bible causes Roam to bog down right now.

## Usage

Compile the program. Right now you'll need to install [Rust](https://www.rust-lang.org/) and compile it yourself.

Once compiled, `xml-bible-to-roam -f bible.xml -b "book name" -c chapter_number` will output the JSON to standard output. 

Example: `xml-bible-to-roam -f ESV.xml -b Joshua -c 11 > Joshua11.json` and then you can just import it with Roam's "Import Files" menu option.

After importing the chapter, I like to add it to a page for the book using a block embed like so: `{{embed: ((type the chapter name here))}}`.
