use log::debug;
use nom::branch::alt;
use nom::bytes::complete::{tag_no_case, take_until};
use nom::character::complete::{alphanumeric0, anychar, multispace0, space0};
use nom::combinator::{map, recognize};
use nom::multi::{many0, many0_count};
use nom::sequence::{delimited, terminated, tuple};
use nom::IResult;
use regex::Regex;
use scraper::{Html, Selector};

fn parse_content(input: &str) -> IResult<&str, &str> {
    alt((parse_strong, take_until("<")))(input)
}

fn parse_td(input: &str) -> IResult<&str, &str> {
    terminated(
        alt((
            delimited(tag_no_case("<td>"), parse_content, tag_no_case("</td>")),
            delimited(tag_no_case("<th>"), parse_content, tag_no_case("</th>")),
        )),
        multispace0,
    )(input)
}

fn parse_strong(input: &str) -> IResult<&str, &str> {
    delimited(
        tag_no_case("<strong>"),
        parse_content,
        tag_no_case("</strong>"),
    )(input)
}

fn parse_two(input: &str) -> IResult<&str, (&str, &str)> {
    tuple((parse_td, parse_td))(input)
}

fn parse_three(input: &str) -> IResult<&str, (&str, &str)> {
    terminated(tuple((parse_td, parse_td)), parse_td)(input)
}

fn parse_alt(input: &str) -> IResult<&str, (&str, &str)> {
    alt((parse_three, parse_two))(input)
}

pub fn parse_tr(input: &str) -> IResult<&str, (&str, &str)> {
    delimited(
        terminated(tag_no_case("<tr>"), multispace0),
        alt((parse_three, parse_two)),
        terminated(tag_no_case("</tr>"), multispace0),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_td() {
        let input = "<td>foo</td>";
        let (_, r) = parse_td(input).unwrap();
        assert_eq!("foo", r)
    }

    #[test]
    fn test_parse_td_with_strong() {
        let input = "<td><strong>foo</strong></td>";
        let (_, r) = parse_td(input).unwrap();
        assert_eq!("foo", r)
    }

    #[test]
    fn test_parse_td_with_strong_1() {
        let input = "<td><strong>Binary Representation</strong></td>";
        let (_, r) = parse_td(input).unwrap();
        assert_eq!("Binary Representation", r)
    }

    #[test]
    fn test_parse_th_with_strong_1() {
        let input = "<th><strong>Binary Representation</strong></th>";
        let (_, r) = parse_td(input).unwrap();
        assert_eq!("Binary Representation", r)
    }

    #[test]
    fn test_parse_two() {
        let input = "<td>foo</td><td>bar</td>";

        let (_, (a, b)) = parse_two(input).unwrap();
        assert_eq!("foo", a);
        assert_eq!("bar", b);
    }

    #[test]
    fn test_parse_three() {
        let input = "<td>foo</td><td>bar</td><td>non</TD>";

        let (_, (a, b)) = parse_three(input).unwrap();
        assert_eq!("foo", a);
        assert_eq!("bar", b);
    }

    #[test]
    fn test_parse_alt() {
        let input = "<td>foo</td><td>bar</td><td>non</td>";

        let r = parse_alt(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("foo", a);
        assert_eq!("bar", b);
    }

    #[test]
    fn test_parse_alt1() {
        let input = "<td>foo</td><td>bar</td>";

        let r = parse_alt(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("foo", a);
        assert_eq!("bar", b);
    }

    #[test]
    fn test_parse_alt11() {
        let input = "<td>foo</td>\n\
        <td>bar</td>";

        let r = parse_alt(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("foo", a);
        assert_eq!("bar", b);
    }

    #[test]
    fn test_parse_tr() {
        let input = "<tr><td>foo</td><td>bar</td><td>non</td></tr>";

        let r = parse_tr(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("foo", a);
        assert_eq!("bar", b);
    }

    #[test]
    fn test_parse_tr1() {
        let input = "<tr>\n\
        <td>foo</td><td>bar</td><td>non</td>\n\
        </tr>";

        let r = parse_tr(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("foo", a);
        assert_eq!("bar", b);
    }

    #[test]
    fn test_parse_tr2() {
        let input = "<tr>\n\
        <td>foo</td><td>bar</td>\n\
        </tr>";

        let r = parse_tr(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("foo", a);
        assert_eq!("bar", b);
    }

    #[test]
    fn test_parse_tr3() {
        let input = "<tr>\n\
        <td>foo</td>\n\
        <td>bar</td>\n\
        <td>non</td>\n\
        </tr>";

        let r = parse_tr(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("foo", a);
        assert_eq!("bar", b);
    }

    #[test]
    fn test_parse_tr_1() {
        let input = r##"<tr><td>94</td><td>Vietnam</td><td>1001 0100</td></tr>"##;

        let r = parse_tr(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("94", a);
        assert_eq!("Vietnam", b);
    }

    #[test]
    fn test_parse_tr_2() {
        let input = r##"<tr><td>94</td>
        <td>Vietnam</td>
        <td>1001 0100</td>
        </tr>"##;

        let r = parse_tr(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("94", a);
        assert_eq!("Vietnam", b);
    }

    #[test]
    fn test_parse_tr_3() {
        let input = r##"<tr><th>SAC(Hexa)</th>
    <th>Country/Geographical Area</th>
    <th>Binary Representation</th>
    </tr>
    "##;

        let r = parse_tr(input);
        dbg!(&r);
        assert!(r.is_ok());

        let (_, (a, b)) = r.unwrap();
        assert_eq!("SAC(Hexa)", a);
        assert_eq!("Country/Geographical Area", b);
    }
}
