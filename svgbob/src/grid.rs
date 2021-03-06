use loc::Loc;
use focus_char::FocusChar;
use optimizer::Optimizer;
use svg_element::SvgElement;
use svg::node::element::SVG;
use svg::Node;
use svg::node::element::{
    Definitions,
    Marker,
    Path as SvgPath,
    Rectangle as SvgRect,
    Style,
    Circle as SvgCircle,
    Group,
};
use element::Element;
use pom::TextInput;
use pom::parser::{sym,none_of};
use settings::Settings;
use unicode_width::UnicodeWidthChar;

#[derive(Debug)]
pub struct Grid {
    pub settings: Settings,
    /// cell value is in string instead of char to accomodate multiple width characters
    /// each line is Vec<String>
    index: Vec<Vec<String>>,
    /// This are text elements that are escaped and are not processed for diagram
    /// matching
    text_elm: Vec<(usize, usize, String)>,
}
impl Grid {
    /// instantiate a grid from input ascii text
    /// Issues:
    /// 1. 2-width, 5 bytes, single character  i.e. 统
    /// 2. 1-width, 2 bytes, single character  i.e. ö
    /// 3. 1-width, 3 bytes, single character  i.e. o͡͡͡
    pub fn from_str(s: &str, settings: &Settings) -> Grid {
        let lines: Vec<&str> = s.lines().collect();
        let mut rows: Vec<Vec<String>> = Vec::with_capacity(lines.len());
        let mut text_elm: Vec<(usize, usize, String)> = vec![];
        for (y, line) in lines.iter().enumerate() {
            let (line, escaped_texts): (String, Vec<(usize, String)>) = exclude_escaped_text(line);
            let mut row: Vec<String> = Vec::with_capacity(line.chars().count());
            for (x, escaped) in escaped_texts {
                text_elm.push((x, y, svg_escape(&escaped)));
            }
            for ch in line.chars() {
                if let Some(1) = ch.width() {
                    row.push(format!("{}", ch));
                } else if let Some(2) = ch.width() {
                    row.push(format!("{}", ch));
                    // HACK: push a blank to the next cell,
                    //in order to make this character twice as
                    // big and aligns the next succeeding characters on
                    // this row
                    row.push(format!("\0"));
                }
                // if zero width char, append it to the previous string
                else if let Some(0) = ch.width() {
                    let prev: Option<String> = row.pop();
                    match prev {
                        Some(mut prev) => {
                            prev.push(ch);
                            row.push(prev);
                        }
                        None => (),
                    }
                }
            }
            rows.push(row);
        }
        let g = Grid {
            settings: settings.clone(),
            index: rows,
            text_elm: text_elm,
        };
        // do the pre processing here
        g.pre_process()
    }


    pub fn rows(&self) -> usize {
        self.index.len()
    }

    /// get the maximum row len
    pub fn columns(&self) -> usize {
        self.index.iter().map(|r| r.len()).max().unwrap_or(0)
    }

    /// get a character at this location
    /// widths are computed since there are
    /// characters that spans 2 columns
    /// and characters that has 0 width
    ///
    pub fn get(&self, loc: &Loc) -> Option<&String> {
        match self.index.get(loc.y as usize) {
            Some(row) => row.get(loc.x as usize),
            None => None,
        }
    }


    fn text(&self, loc: &Loc) -> &str {
        match self.get(loc) {
            Some(ref s) => s,
            None => ""
        }
    }

    /// get the focus char at this location
    pub fn get_focuschar(&self, loc: &Loc) -> FocusChar {
        FocusChar::new(&loc, self)
    }

    /// process the enhancing of circle elements
    /// this should be called before other elements are extracted from the grid
    fn get_enhance_circle_elements(&self) -> (Vec<Vec<Vec<Element>>>, Vec<Loc>){
        let mut rows: Vec<Vec<Vec<Element>>> = Vec::with_capacity(self.index.len());
        let mut all_consumed_loc: Vec<Loc> = vec![];
        for (y,line) in self.index.iter().enumerate() {
            let mut row: Vec<Vec<Element>> = Vec::with_capacity(line.len());
            for (x,_cell) in line.iter().enumerate() {
                let loc = Loc::new(x as i32, y as i32);
                let focus_char = self.get_focuschar(&loc);
                let (cell_elements, consumed_loc) = focus_char.get_enhance_circle_elements();
                all_consumed_loc.extend(consumed_loc);
                row.push(cell_elements);
            }
            rows.push(row);
        }
        (rows, all_consumed_loc)
    }

    /// process the enhanced circle elements first
    /// then process the generic enhancements
    fn get_enhance_elements(&self) -> (Vec<Vec<Vec<Element>>>, Vec<Loc>){
        let (enhanced_circle_elm, circle_consumed_loc) = self.get_enhance_circle_elements();
        let mut rows: Vec<Vec<Vec<Element>>> = Vec::with_capacity(self.index.len());
        rows.extend(enhanced_circle_elm);
        let mut all_consumed_loc: Vec<Loc> = vec![];
        for (y,line) in self.index.iter().enumerate() {
            let mut row: Vec<Vec<Element>> = Vec::with_capacity(line.len());
            for (x,_cell) in line.iter().enumerate() {
                let loc = Loc::new(x as i32, y as i32);
                if !circle_consumed_loc.contains(&loc){
                    let focus_char = self.get_focuschar(&loc);
                    let (cell_elements, consumed_loc) = focus_char.get_enhance_elements();
                    all_consumed_loc.extend(consumed_loc);
                    row.push(cell_elements);
                }
            }
            rows.push(row);
        }
        all_consumed_loc.extend(circle_consumed_loc);
        (rows, all_consumed_loc)
    }

    /// vector of each elements arranged in rows x columns
    /// returns all the elements and the consumed location
    fn get_all_elements(&self) -> Vec<Vec<Vec<Element>>>{
        let (enhanced_elms, enhance_consumed_locs) = self.get_enhance_elements();
        let mut rows: Vec<Vec<Vec<Element>>> = Vec::with_capacity(self.index.len());
        rows.extend(enhanced_elms);
        for (y, line) in self.index.iter().enumerate() {
            let mut row: Vec<Vec<Element>> = Vec::with_capacity(line.len());
            for (x, _cell) in line.iter().enumerate() {
                let loc = Loc::new(x as i32, y as i32);
                if !enhance_consumed_locs.contains(&loc) {
                    let focus_char = self.get_focuschar(&loc);
                    let cell_elements = focus_char.get_elements();
                    row.push(cell_elements);
                }
            }
            rows.push(row);
        }
        rows
    }


    fn get_escaped_text_elements(&self) -> Vec<Element> {
        self.text_elm
            .iter()
            .map(|&(x, y, ref text)| Element::Text(Loc::new(x as i32, y as i32), text.to_owned()))
            .collect()
    }

    /// each component has its relative location retain
    /// use this info for optimizing svg by checking closest neigbor
    fn get_svg_nodes(&self) -> Vec<Vec<SvgElement>> {
        let mut grouped_nodes = vec![];
        let mut elements= self.get_all_elements();
        let text_elm = self.get_escaped_text_elements();
        elements.push(vec![text_elm]);
        let optimizer = Optimizer::new(elements);
        let optimized_elements = optimizer.optimize(&self.settings);
        for group in optimized_elements {
            let mut svg_group = vec![];
            for elem in group{
                let element: SvgElement = elem.to_svg(&self.settings);
                svg_group.push(element);
            }
            grouped_nodes.push(svg_group);
        }
        grouped_nodes
    }


    pub fn get_size(&self) -> (f32, f32) {
        let width = self.settings.text_width * self.columns() as f32;
        let height = self.settings.text_height * self.rows() as f32;
        (width, height)
    }

    /// get the generated svg according to the settings specified
    pub fn get_svg(&self) -> SVG {
        let group_nodes = self.get_svg_nodes();
        let (width, height) = self.get_size();
        let mut svg = SVG::new();

        if let Some(ref id) = self.settings.id {
            svg.assign("id", id.to_owned());
        }
        if let Some(ref class) = self.settings.class {
            svg.assign("class", class.to_owned());
        }
        svg.assign("font-size", self.settings.font_size);
        svg.assign("font-family", self.settings.font_family.to_owned());
        svg.assign("width", width);
        svg.assign("height", height);


        svg.append(get_defs());
        svg.append(get_styles(&self.settings));


        let rect = SvgRect::new()
            .set("x", 0)
            .set("y", 0)
            .set("fill", self.settings.background_color.to_string())
            .set("width", width)
            .set("height", height);
        svg.append(rect);

        for group in group_nodes {
            let mut svg_group = Group::new();
            for node in group{
                match node {
                    SvgElement::Circle(circle) => {
                        svg_group.append(circle);
                    }
                    SvgElement::Line(line) => {
                        svg_group.append(line);
                    }
                    SvgElement::Path(path) => {
                        svg_group.append(path);
                    }
                    SvgElement::Text(text) => {
                        svg_group.append(text);
                    }
                }
            }
            svg.append(svg_group);
        }
        svg
    }
    /// traverse each element of the grid and swap characters as needed
    fn pre_process(&self) -> Self {
        let mut new_index: Vec<Vec<String>> = vec![];
        for (y,line) in self.index.iter().enumerate(){
            let mut row: Vec<String> = vec![];
            for (x,_) in line.iter().enumerate(){
                let loc = &Loc::new(x as i32, y as i32);
                let swap = self.swap_char(loc);
                row.push(swap.to_string());
            }
            new_index.push(row);
        }
        Grid{
            settings: self.settings.clone(),
            index: new_index,
            text_elm: self.text_elm.clone()
        }
    }

    /// swap characters  - - - with ~~~~~
    fn swap_char(&self, loc: &Loc) -> &str {
        let cell = self.text(loc);

        let left = self.text(&loc.left());
        let left2 = self.text(&loc.in_left(2));
        let left3 = self.text(&loc.in_left(3));
        let left4 = self.text(&loc.in_left(4));
        let right = self.text(&loc.right());
        let right2 = self.text(&loc.in_right(2));
        let right3 = self.text(&loc.in_right(3));
        let right4 = self.text(&loc.in_right(4));
        // [- - -]
        //  ^
        // if `-` and right is ` ` and right(2) is `-` right(3) is ` ` and right(4) is `-`
        if (cell == "-" && right == " " && right2 == "-" && right3 == " " && right4 == "-")
        // [- - -]
        //   ^
        // if ` `  and left  is `-` and right is `-` and right(2) is ` ` and right(3) is `-`
        || (cell == " " && left == "-" && right == "-" && right2 == " " && right3 == "-")
        // [- - -]
        //    ^
        // if `-` , and left  is ` ` and right is ` ` and left(2) is `-`  and right(2) is `-` 
        || (cell == "-" && left == " " && right == " " && left2 == "-" && right2 == "-")
        // [- - -]
        //     ^
        //  if ` `, and left is `-` and right is `-` and left(2) is ` ` and left(3) is `-`  
        || (cell == " " && left == "-" && right == "-" && left2 == " " && left3 == "-")
        // [- - -]
        //      ^
        //  if `-`, and left  is ` ` and left(2) is `-` and left(3) is ` ` and left(4) is `-`  
        || (cell == "-" && left == " " && left2 == "-" && left3 == " " && left4 == "-")
        {
            "~"
        }
        else{
           cell
        }
    }

}

fn get_defs() -> Definitions {
    let mut defs = Definitions::new();
    defs.append(arrow_marker());
    defs.append(circle_marker());
    defs.append(square_marker());
    defs.append(open_circle_marker());
    defs.append(big_open_circle_marker());
    defs
}

fn get_styles(settings: &Settings) -> Style {
    let style = format!(
        r#"
    line, path {{
      stroke: {stroke_color};
      stroke-width: {stroke_width};
      stroke-opacity: 1;
      fill-opacity: 1;
      stroke-linecap: round;
      stroke-linejoin: miter;
    }}
    line.dashed {{
        stroke-dasharray: 5;
    }}
    circle.solid {{
      fill:{stroke_color};
      stroke: black;
      stroke-width: {stroke_width};
      stroke-opacity: 1;
      fill-opacity: 1;
      stroke-linecap: round;
      stroke-linejoin: miter;
    }}
    circle.open {{
      fill:none;
      stroke: black;
      stroke-width: {stroke_width};
      stroke-opacity: 1;
      fill-opacity: 1;
      stroke-linecap: round;
      stroke-linejoin: miter;
    }}
    tspan.head{{
        fill: none;
        stroke: none;
    }}
    "#,
        stroke_width = settings.stroke_width, 
        stroke_color = &settings.stroke_color, 
    );
    Style::new(style)
}

fn arrow_marker() -> Marker {
    let mut marker = Marker::new()
        .set("id", "triangle")
        .set("viewBox", "0 0 50 20")
        .set("refX", 15)
        .set("refY", 10)
        .set("markerWidth", 10)
        .set("markerHeight", 10)
        .set("orient", "auto");

    let path = SvgPath::new().set("d", "M 0 0 L 30 10 L 0 20 z");
    marker.append(path);
    marker
}


///   <marker id="dot" viewBox="0 0 10 10" refX="5" refY="5"
///        markerWidth="5" markerHeight="5">
///      <circle cx="5" cy="5" r="5" fill="red" />
///    </marker>
fn circle_marker() -> Marker {
    let mut marker = Marker::new()
        .set("id", "circle")
        .set("viewBox", "0 0 20 20")
        .set("refX", 10)
        .set("refY", 10)
        .set("markerWidth", 5)
        .set("markerHeight", 5);

    let circle = SvgCircle::new()
        .set("cx",10)
        .set("cy",10)
        .set("r",8)
        .set("fill","black");
    marker.append(circle);
    marker
}

fn square_marker() -> Marker {
    let mut marker = Marker::new()
        .set("id", "square")
        .set("viewBox", "0 0 20 20")
        .set("refX", 10)
        .set("refY", 10)
        .set("markerWidth", 5)
        .set("markerHeight", 5);

    let square = SvgRect::new()
        .set("x",0)
        .set("y",0)
        .set("width",20)
        .set("height",20)
        .set("fill","black");
    marker.append(square);
    marker
}

fn open_circle_marker() -> Marker {
    let mut marker = Marker::new()
        .set("id", "open_circle")
        .set("viewBox", "0 0 20 20")
        .set("refX", 10)
        .set("refY", 10)
        .set("markerWidth", 10)
        .set("markerHeight", 10);

    let circle = SvgCircle::new()
        .set("cx",10)
        .set("cy",10)
        .set("r",4)
        .set("stroke","black")
        .set("stroke-width",2)
        .set("fill","white");
    marker.append(circle);
    marker
}
fn big_open_circle_marker() -> Marker {
    let mut marker = Marker::new()
        .set("id", "big_open_circle")
        .set("viewBox", "0 0 40 40")
        .set("refX", 20)
        .set("refY", 20)
        .set("markerWidth", 20)
        .set("markerHeight", 20);

    let circle = SvgCircle::new()
        .set("cx",20)
        .set("cy",20)
        .set("r",6)
        .set("stroke","black")
        .set("stroke-width",2)
        .set("fill","white");
    marker.append(circle);
    marker
}
//copied from https://github.com/rust-lang/rust/blob/master/src/librustdoc/html/escape.rs
//just adding for \0
pub fn svg_escape(arg: &str) -> String {
    use std::fmt;

    /// Wrapper struct which will emit the HTML-escaped version of the contained
    /// string when passed to a format string.
    pub struct Escape<'a>(pub &'a str);

    impl<'a> fmt::Display for Escape<'a> {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            // Because the internet is always right, turns out there's not that many
            // characters to escape: http://stackoverflow.com/questions/7381974
            let Escape(s) = *self;
            let pile_o_bits = s;
            let mut last = 0;
            for (i, ch) in s.bytes().enumerate() {
                match ch as char {
                    '<' | '>' | '&' | '\'' | '"' | '\0' => {
                        fmt.write_str(&pile_o_bits[last..i])?;
                        let s = match ch as char {
                            '>' => "&gt;",
                            '<' => "&lt;",
                            '&' => "&amp;",
                            '\'' => "&#39;",
                            '"' => "&quot;",
                            '\0' => "",
                            _ => unreachable!(),
                        };
                        fmt.write_str(s)?;
                        last = i + 1;
                    }
                    _ => {}
                }
            }

            if last < s.len() {
                fmt.write_str(&pile_o_bits[last..])?;
            }
            Ok(())
        }
    };
    let escaped = Escape(arg);
    format!("{}", escaped)
}

fn exclude_escaped_text(line: &str) -> (String, Vec<(usize, String)>) {
    let mut input = TextInput::new(line);
    let parsed = line_parse().parse(&mut input);
    let mut buffer = String::new();
    let mut text_elm: Vec<(usize, String)> = vec![];
    if let Ok(parsed) = parsed {
        let mut index = 0;
        if !parsed.is_empty() {
            for (start, end) in parsed {
                let escaped = &line[start + 1..end];
                let recons = &line[index..start];
                text_elm.push((start, escaped.to_string()));
                buffer.push_str(recons);
                buffer.push_str(&" ".repeat(end + 1 - start));
                index = end + 1;
            }
            buffer.push_str(&line[index..line.len()]);
        } else {
            buffer.push_str(line);
        }
    }
    (buffer, text_elm)
}

#[test]
fn test_escaped_string() {
    let input3 = r#"The "qu/i/ck" brown "fox\"s" jumps over the lazy "do|g""#;
    let mut raw3 = TextInput::new(input3);
    let output3 = line_parse().parse(&mut raw3);
    println!("output3: {:?}", output3);
    //assert_eq!(Ok(vec![(4, 12), (20, 27), (49, 54)]), output3);
    let mut matches = vec![];
    let mut recons = String::new();
    let mut text_elm: Vec<(usize, String)> = vec![];
    let mut index = 0;
    if let Ok(output) = output3 {
        for (start, end) in output {
            println!("matches: {}", &input3[start..end + 1]);
            matches.push(input3[start..end + 1].to_string());
            let slice = &input3[index..start];
            recons.push_str(slice);
            recons.push_str(&" ".repeat(end + 1 - start));
            text_elm.push((start, input3[start + 1..end].to_string()));
            index = end + 1;
        }
    }
    println!("input3: {}", input3);
    println!("recons: {}", recons);
    println!("escaped: {:?}", text_elm);
    assert_eq!(vec![r#""qu/i/ck""#, r#""fox\"s""#, r#""do|g""#], matches);
    assert_eq!(input3.len(), recons.len());
}

#[test]
fn test_escaped_multiline_string() {
    let input3 = r#"The "qu/i/ck brown fox \njumps over the lazy do|g""#;
    let mut raw3 = TextInput::new(input3);
    let output3 = line_parse().parse(&mut raw3);
    println!("output3: {:?}", output3);
    assert_eq!(Ok(vec![(4, 49)]), output3);
    let mut matches = vec![];
    let mut recons = String::new();
    let mut text_elm: Vec<(usize, String)> = vec![];
    let mut index = 0;
    if let Ok(output) = output3 {
        for (start, end) in output {
            println!("matches: {}", &input3[start..end + 1]);
            matches.push(input3[start..end + 1].to_string());
            let slice = &input3[index..start];
            recons.push_str(slice);
            recons.push_str(&" ".repeat(end + 1 - start));
            text_elm.push((start, input3[start + 1..end].to_string()));
            index = end + 1;
        }
    }
    println!("input3: {}", input3);
    println!("recons: {}", recons);
    println!("escaped: {:?}", text_elm);
    assert_eq!(
        vec![r#""qu/i/ck brown fox \njumps over the lazy do|g""#],
        matches
    );
    assert_eq!(input3.len(), recons.len());
}

fn escape_string() -> pom::parser::Parser<'static, char, (usize, usize)> {
    let escape_sequence = sym('\\') * sym('"'); //escape sequence \"
    let char_string = escape_sequence | none_of("\"");
    let escaped_string_end = sym('"') * char_string.repeat(0..).pos() - sym('"');
    none_of("\"").repeat(0..).pos() + escaped_string_end - none_of("\"").repeat(0..).discard()
}

fn line_parse() -> pom::parser::Parser<'static, char, Vec<(usize, usize)>> {
    escape_string().repeat(0..)
}

#[cfg(test)]
mod test_lib {
    use super::Grid;
    use super::Loc;
    use super::Settings;

    #[test]
    fn test_grid() {
        let g = Grid::from_str("a统öo͡͡͡", &Settings::compact());
        println!("{:?}", g.index);
        assert_eq!(
            g.index,
            vec![vec![
                "a".to_string(),
                "统".to_string(),
                "\u{0}".to_string(),
                "ö".to_string(),
                "o͡͡͡".to_string(),
            ]]
        );
    }

    #[test]
    fn test_text_in_range() {
        let txt = "
1234567890
abcdefghij
klmnopqrst
uvwxyz1234
567890abcd
        ";
        let g = Grid::from_str(txt, &Settings::compact());
        let loc = Loc::new(4, 3); // at 'o'
        let (loc1, loc2) = loc.get_range(2, 1);
        let text = g.get_text_in_range(&loc1, &loc2);
        assert_eq!(
            text,
            vec![
                vec![
                    Some(&"c".to_string()),
                    Some(&"d".to_string()),
                    Some(&"e".to_string()),
                    Some(&"f".to_string()),
                    Some(&"g".to_string()),
                ],
                vec![
                    Some(&"m".to_string()),
                    Some(&"n".to_string()),
                    Some(&"o".to_string()),
                    Some(&"p".to_string()),
                    Some(&"q".to_string()),
                ],
                vec![
                    Some(&"w".to_string()),
                    Some(&"x".to_string()),
                    Some(&"y".to_string()),
                    Some(&"z".to_string()),
                    Some(&"1".to_string()),
                ],
            ]
        );
    }

    #[test]
    fn test_to_string() {
        let txt = "The quick brown fox
jumps over
     the lazy dog.
       ]";
        let g = Grid::from_str(txt, &Settings::compact());
        assert_eq!(txt, &*g.to_string());
    }
    #[test]
    fn test_to_trimmed_string() {
        let txt = "

The quick brown fox

jumps over

     the lazy dog.

        ";
        let g = Grid::from_str(txt, &Settings::compact());
        assert_eq!(txt, &*g.to_string());
    }

    #[test]
    fn test_insert_text() {
        let txt = "1234567890
abcdefghij
klmnopqrst
uvwxyz1234
567890abcd";
        let mut g = Grid::from_str(txt, &Settings::compact());
        g.put(&Loc::new(-1, -1), "-");
        let expected = "-
 1234567890
 abcdefghij
 klmnopqrst
 uvwxyz1234
 567890abcd";
        assert_eq!(expected, &*g.to_string());
    }

    #[test]
    fn test_insert_text_after() {
        let txt = "\
1234567890
abcdefghij
klmnopqrst
uvwxyz1234
567890abcd\
";
        let mut g = Grid::from_str(txt, &Settings::compact());
        g.put(&Loc::new(11, 5), "1");
        let expected = "\
1234567890
abcdefghij
klmnopqrst
uvwxyz1234
567890abcd
           1\
";
        assert_eq!(expected, &*g.to_string());
    }

    #[test]
    fn test_slash0_space() {
        let txt = "件hello统";
        let g = Grid::from_str(txt, &Settings::compact());
        let s = g.to_string();
        assert_eq!(txt, s);
    }

}

#[cfg(test)]
mod test {
    use super::super::Loc;
    use super::super::Settings;
    use super::FocusChar;
    use super::Grid;
    use fragments::Direction::*;
    use properties::Location;

    use fragments::Block::{O, U, Y};

    #[test]
    fn test_adjascent() {
        let g = Grid::from_str("a统öo͡͡͡", &Settings::compact());
        let fc = FocusChar::new(&Loc::new(1, 0), &g);
        println!("{:?}", fc);
        assert!(fc.left().is('a'));
        assert!(fc.right().right().is('ö'));
    }

    #[test]
    fn test100() {
        //  ._
        let g = Grid::from_str(".-", &Settings::separate_lines());
        let fc = FocusChar::new(&Loc::new(0, 0), &g);
        let (frags, _consumed) = fc.get_fragments();
        println!("frags: {:?}", frags);
        assert!(fc.is_intensified(&O));
        assert!(fc.can_be_strong_block(&O));
    }

    #[test]
    fn test_location() {
        //  ._
        let g = Grid::from_str(".-", &Settings::separate_lines());
        let fc = FocusChar::new(&Loc::new(0, 0), &g);
        let (_frags, _consumed) = fc.get_fragments();
        let go_right = fc.from_location(&Location::go(Right));
        let right = fc.right();
        let right2 = fc.in_right(2);
        let mut right2_loop = fc.clone();
        for _ in 0..2 {
            right2_loop = right2_loop.in_right(1);
        }
        println!("in right 2: {:?}", right2.loc);
        println!("in right 2 loop: {:?}", right2_loop.loc);
        assert_eq!(right2.loc, right2_loop.loc);
        assert_eq!(go_right.loc, right.loc);
    }

    #[test]
    fn test_loc() {
        let g = Grid::from_str("", &Settings::separate_lines());
        let fc = FocusChar::new(&Loc::new(0, 0), &g);
        let right = fc.right();
        let in_right = fc.in_right(1);
        assert_eq!(Loc::new(1, 0), right.loc);
        assert_eq!(Loc::new(1, 0), in_right.loc);
    }

    #[test]
    fn test1() {
        //  ._
        let g = Grid::from_str("._", &Settings::separate_lines());
        let fc = FocusChar::new(&Loc::new(0, 0), &g);
        println!("focus char: {:#?}", fc);
        let (frags, _consumed) = fc.get_fragments();
        println!("frags: {:?}", frags);
        assert!(!fc.is_intensified(&U));
        assert!(fc.is_intensified(&Y));
    }
    #[test]
    fn test2() {
        //  ._
        let g = Grid::from_str("._", &Settings::separate_lines());
        let fc = FocusChar::new(&Loc::new(1, 0), &g);
        println!("focus char: {:#?}", fc);
        let (frags, _consumed) = fc.get_fragments();
        println!("frags: {:?}", frags);
        assert!(!fc.is_intensified(&Y));
        assert!(!fc.is_intensified(&U));
        assert!(fc.can_be_strong_block(&Y));
        assert!(fc.can_be_strong_block(&U));
    }

    #[test]
    fn test_no_character() {
        use properties::Properties;
        use {FocusChar, Grid, Loc, Settings};

        let g = Grid::from_str(".l", &Settings::separate_lines());
        let fc = FocusChar::new(&Loc::new(1, 0), &g);
        println!("focus char: {:#?}", fc);
        let character = fc.ch.get_characteristic();
        println!("character: {:#?}", character);
        assert!(character.is_none());
    }

    #[test]
    fn test_has_character() {
        use properties::Properties;
        use {FocusChar, Grid, Loc, Settings};

        let g = Grid::from_str(".╦", &Settings::separate_lines());
        let fc = FocusChar::new(&Loc::new(1, 0), &g);
        println!("focus char: {:#?}", fc);
        let character = fc.ch.get_characteristic();
        println!("character: {:#?}", character);
        assert!(character.is_some());
    }
}
