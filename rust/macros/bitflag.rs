// SPDX-License-Identifier: GPL-2.0

use crate::helpers::*;
use proc_macro::{token_stream, Delimiter, TokenStream, TokenTree};

/// parse the groups_of_incompatible field passed to the [crate::bitflag] macro
fn expect_incompat_groups(it: &mut token_stream::IntoIter) -> Vec<(String, Vec<(String, String)>)> {
    let group = expect_group(it);
    assert_eq!(group.delimiter(), Delimiter::Brace);
    let mut values = Vec::new();
    let mut it = group.stream().into_iter();
    let mut seen_keys: Vec<String> = Vec::new();
    loop {
        let key = match it.next() {
            Some(TokenTree::Ident(ident)) => ident.to_string(),
            Some(_) => panic!(
                "Keys of groups: Expected Ident or end. Last valid key was \"{:?}\"",
                seen_keys.last()
            ),
            None => break,
        };

        if seen_keys.contains(&key) {
            panic!(
                "Duplicated key \"{}\". Keys can only be specified once.",
                key
            );
        }

        assert_eq!(
            try_punct(&mut it),
            Some(':'),
            "after key {}, expected ':'",
            key
        );

        let value = expect_incompat_group(&mut it);
        values.push((key.clone(), value.clone()));

        assert_eq!(
            try_punct(&mut it),
            Some(','),
            "after value {:?}, expected ':'",
            value
        );
        seen_keys.push(key);
    }
    values
}

/// parse a specific group from the set of groups in groups_of_incompatible. This is called in [expect_incompat_groups]
fn expect_incompat_group(it: &mut token_stream::IntoIter) -> Vec<(String, String)> {
    let group = expect_group(it);
    assert_eq!(group.delimiter(), Delimiter::Brace);
    let mut values = Vec::new();
    let mut it = group.stream().into_iter();

    let mut seen_keys: Vec<String> = Vec::new();
    loop {
        let mut key = match it.next() {
            Some(TokenTree::Ident(ident)) => ident.to_string(),
            Some(_) => panic!(
                "flag name: Expected Ident or end. Last valid key was \"{:?}\"",
                seen_keys.last()
            ),
            None => break,
        };
        if !key.is_ascii() {
            panic!("\"{}\" is not an ASCII string", key);
        } else {
            key.make_ascii_lowercase();
        }

        if seen_keys.contains(&key) {
            panic!(
                "Duplicated key \"{}\". Keys can only be specified once.",
                key
            );
        }
        assert_eq!(
            try_punct(&mut it),
            Some(':'),
            "after key {}, expected ':'",
            key
        );

        let mut value = "".to_string();
        let current_value = value.clone();
        let path_test = |it: &mut token_stream::IntoIter| match try_punct(it) {
            Some(',') => None,
            Some(':') => match try_punct(it) {
                Some(':') => {
                    let next_val = try_ident(it).unwrap_or_else(|| {
                        panic!(
                            "flag value for flag \"{}\": Expected Ident as part of path",
                            key
                        )
                    });
                    Some(next_val)
                }
                _ => panic!("expected two colons, got only one"),
            },
            _ => panic!(
                "after {}, expected either a colon path or a comma",
                current_value
            ),
        };

        match it.next() {
            Some(TokenTree::Ident(ident)) => {
                value = ident.to_string();
                while let Some(next_path_val) = path_test(&mut it) {
                    value.push_str("::");
                    value.push_str(next_path_val.as_str());
                }
            }
            Some(TokenTree::Literal(litteral)) => {
                value = litteral.to_string();
                assert_eq!(
                    try_punct(&mut it),
                    Some(','),
                    "after key {}, expected ','",
                    key);
            },
            other => panic!("flag value for flag \"{}\": Expected Ident, Ident with path, or litteral, instead got: {other:?}", key)
        }

        values.push((key.clone(), value));

        seen_keys.push(key);
    }
    values
}

#[derive(Debug, Default)]
struct BitflagInfo {
    bitflag_name: String,
    bitflag_type: String,
    bitflag_groups: Vec<(String, Vec<(String, String)>)>,
}

impl BitflagInfo {
    fn parse(it: &mut token_stream::IntoIter) -> Self {
        let it = it.into_iter();

        let mut info: BitflagInfo = Default::default();

        const REQUIRED_KEYS: &[&str] = &["name", "type", "groups_of_incompatible"];
        let mut seen_keys = Vec::new();

        loop {
            let key = match it.next() {
                Some(TokenTree::Ident(ident)) => ident.to_string(),
                Some(_) => panic!("Expected Ident or end"),
                None => break,
            };

            if seen_keys.contains(&key) {
                panic!(
                    "Duplicated key \"{}\". Keys can only be specified once.",
                    key
                );
            }

            assert_eq!(expect_punct(it), ':');

            match key.as_str() {
                "type" => info.bitflag_type = try_ident(it).expect("type: expects an ident"),
                "name" => info.bitflag_name = try_ident(it).expect("name: expects an ident"),
                "groups_of_incompatible" => info.bitflag_groups = expect_incompat_groups(it),
                _ => panic!(
                    "Unknown key \"{}\". Valid top level keys are: {:?}.",
                    key, REQUIRED_KEYS
                ),
            }

            assert_eq!(
                try_punct(it),
                Some(','),
                "expected comma after key-value pair {}:...",
                key.clone()
            );

            seen_keys.push(key);
        }

        expect_end(it);

        for key in REQUIRED_KEYS {
            if !seen_keys.iter().any(|e| e == key) {
                panic!("Missing required key \"{}\".", key);
            }
        }

        let mut ordered_keys: Vec<&str> = Vec::new();
        for key in REQUIRED_KEYS {
            if seen_keys.iter().any(|e| e == key) {
                ordered_keys.push(key);
            }
        }

        if seen_keys != ordered_keys {
            panic!(
                "Keys are not ordered as expected. Order them like: {:?}.",
                ordered_keys
            );
        }

        info
    }
}
/// the bitflag macro. Parses the token stream into a BitflagInfo struct, then uses it to generate
/// the appropriate Rust code
pub(crate) fn bitflag_and_builder(ts: TokenStream) -> TokenStream {
    let mut it = ts.into_iter();
    let info = BitflagInfo::parse(&mut it);
    let name = info.bitflag_name.clone();
    let n = info.bitflag_groups.len();

    let missing_generics: Vec<String> = info
        .bitflag_groups
        .iter()
        .map(|(group_name, _group)| format!("crate::bitflag::Missing<{group_name}>"))
        .collect();

    fn struct_from_group((group_name, _group): &(String, Vec<(String, String)>)) -> String {
        format!(
            "
    #[derive(Debug)]
    pub struct {group_name};"
        )
    }

    let generics: Vec<String> = (0..n).map(|k| format!("S{k}")).collect();
    let groups_as_structs = info
        .bitflag_groups
        .iter()
        .map(struct_from_group)
        .collect::<Vec<String>>();

    let missing_impls: Vec<String> = info
        .bitflag_groups
        .iter()
        .enumerate()
        .map(|(k, (group_name, group))| {
            let mut left_generics = generics.clone();
            left_generics.remove(k);
            let mut right_generics = generics.clone();
            right_generics[k] = format!("crate::bitflag::Missing<{group_name}>");
            let mut right_generics_method = generics.clone();
            right_generics_method[k] = format!("crate::bitflag::Valid<{group_name}>");

            let withs = group.iter().map(|(key, _value)| {
                format!(
                    "
        pub fn with_{key}(self) -> {name}Builder<{0}> {{
            let mut b: {name}Builder<{0}> =
                unsafe {{ core::mem::transmute(self) }};
            b.set_{key}();
            b
        }}",
                    right_generics_method.join(", ")
                )
            });

            format!(
                "
    impl<{0}> {name}Builder<{1}> {{
{2}
    }}",
                left_generics.join(","),
                right_generics.join(", "),
                withs.collect::<Vec<String>>().join("\n")
            )
        })
        .collect();

    let valid_impls: Vec<String> = info
        .bitflag_groups
        .iter()
        .enumerate()
        .map(|(k, (group_name, group))| {
            let mut left_generics = generics.clone();
            left_generics.remove(k);
            let mut right_generics = generics.clone();
            right_generics[k] = format!("crate::bitflag::Valid<{group_name}>");

            let setters = group.iter().map(|(key, value)| {
                format!(
                    "
        pub fn set_{key}(&mut self){{
            self.flags[{k}] = Some({value});
        }}"
                )
            });

            format!(
                "
    impl<{0}> {name}Builder<{1}> {{
{2}
    }}",
                left_generics.join(","),
                right_generics.join(", "),
                setters.collect::<Vec<String>>().join("\n")
            )
        })
        .collect();

    let valid_generics: Vec<String> = info
        .bitflag_groups
        .iter()
        .map(|(group_name, _group)| format!("crate::bitflag::Valid<{group_name}>"))
        .collect();
    let try_from_inner = info
        .bitflag_groups
        .iter()
        .map(|(_group_name, group)| {
            format!(
                "
            matched = false;

            for flag in [{}] {{
                if (flag & to_process) == flag {{
                    matched = true;
                    to_process -= flag;
                    if flag > 0 {{
                        break;
                    }}
                }}
            }}
            if !matched {{
                return Err(value);
            }}",
                group
                    .iter()
                    .map(|(_k, v)| v.clone())
                    .collect::<Vec<String>>()
                    .join(",")
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    let try_from_impl = format!(
        "impl TryFrom<<{name} as BitFlag>::Bits> for {name} {{
        type Error = <{name} as BitFlag>::Bits;

        fn try_from(value: <{name} as BitFlag>::Bits) -> Result<Self, Self::Error> {{
            let mut to_process = value.clone();

            let mut {}

            if to_process == 0 {{
                return Ok(Self(value));
            }}
            return Err(value)
        }}
    }}",
        try_from_inner
    );
    // panic!(
    //     "{:?}\n\n{}",
    //     info,
    format!(
        "
    #[derive(Debug, PartialEq)]
    pub struct {name}({type});
    impl BitFlag for {name} {{
        type Bits = {type};

        fn bits(&self) -> Self::Bits {{
            self.0
        }}
    }}

    impl {name} {{
        pub fn builder() -> {name}Builder<{missing_generics}> {{
            {name}Builder {{
                flags: Default::default(),
                t: core::marker::PhantomData,
            }}
        }}
    }}

    #[derive(Debug)]
    #[repr(C)]
    pub struct {name}Builder<{generics}> {{
        flags: [Option<{type}>; {n}],
        t: core::marker::PhantomData<({generics})>,
    }}

    {groups_as_structs}

    {valid_impls}    

    {missing_impls}

    {try_from_impl}

    impl crate::bitflag::ConstrainedFlagBuilder<{name}> for {name}Builder<{valid_generics}> {{
        fn build(self) -> {name} {{
            {name}(self.flags.iter().flatten().sum::<{type}>())
        }}
    }}
",
        type=info.bitflag_type,
        missing_generics = missing_generics.join(", "),
        generics = generics.join(", "),
        groups_as_structs = groups_as_structs.join("\n"),
        valid_impls=valid_impls.join("\n"),
       missing_impls = missing_impls.join("\n"),
       valid_generics = valid_generics.join(", ")
    )
    // )
    .parse()
    .expect("Error parsing formatted string into token stream.")
}

#[derive(Debug, Default)]
struct TrueBitflagInfo {
    bitflag_name: String,
    bitflag_type: String,
    bitflag_options: Vec<(String, String, String)>,
}

/// parse a specific group from the set of groups in groups_of_incompatible. This is called in [expect_incompat_groups]
fn expect_ternary_group(it: &mut token_stream::IntoIter) -> Vec<(String, String, String)> {
    let group = expect_group(it);
    assert_eq!(group.delimiter(), Delimiter::Brace);
    let mut values = Vec::new();
    let mut it = group.stream().into_iter();

    let mut seen_keys: Vec<String> = Vec::new();
    loop {
        let mut key = match it.next() {
            Some(TokenTree::Ident(ident)) => ident.to_string(),
            Some(_) => panic!(
                "flag name: Expected Ident or end. Last valid key was \"{:?}\"",
                seen_keys.last()
            ),
            None => break,
        };
        if !key.is_ascii() {
            panic!("\"{}\" is not an ASCII string", key);
        } else {
            key.make_ascii_lowercase();
        }

        if seen_keys.contains(&key) {
            panic!(
                "Duplicated key \"{}\". Keys can only be specified once.",
                key
            );
        }
        assert_eq!(
            try_punct(&mut it),
            Some(':'),
            "after key {}, expected ':'",
            key
        );

        let parse_ternary = |it: &mut token_stream::IntoIter| -> (String, String) {
            let (mut value_true, mut value_false) = Default::default();
            let mut end_loop = false;
            let mut part: &mut String = &mut value_true;

            for _i in 0..2 {
                match it.next() {
                    Some(TokenTree::Literal(literal)) => {
                        part.push_str(&literal.to_string());
                        if let Some(TokenTree::Punct(punct)) = it.next() {
                            let p_char = punct.as_char();
                            if end_loop {
                                assert_eq!(p_char,',', "flag value for flag \"{key}\": Expected end of ternary comma ','");
                            } else {
                                assert_eq!(
                                    p_char, ':',
                                    "flag value for flag \"{key}\": Expected ternary operator ':'"
                                );
                                assert_eq!(punct.spacing(),proc_macro::Spacing::Alone, "flag value for flag \"{key}\": Expected well spaced ternary operator ' : '");
                                end_loop = true;
                                part = &mut value_false;
                                continue;
                            }
                        }
                    }

                    Some(TokenTree::Ident(ident)) => {
                        part.push_str(&ident.to_string());
                        loop {
                            if let Some(TokenTree::Punct(punct)) = it.next() {
                                match punct.as_char() {
                                    ',' => {
                                        if end_loop {
                                            break;
                                        } else {
                                            panic!("comma before end of ternary parsing");
                                        }
                                    }

                                    ':' => {
                                        if punct.spacing() == proc_macro::Spacing::Alone {
                                            if end_loop {
                                                panic!("unexpected lonely colon");
                                            } else {
                                                end_loop = true;

                                                part = &mut value_false;

                                                break;
                                            }
                                        } else {
                                            assert_eq!(
                                                Some(':'),
                                                try_punct(it),
                                                "Expected second colon as part of path"
                                            );
                                            let next_val = try_ident(it).unwrap_or_else(|| panic!("flag value for flag \"{key}\": Expected Ident as part of path"));
                                            part.push_str("::");
                                            part.push_str(&next_val);
                                        }
                                    }
                                    _ => panic!("unhandled punction while parsing an ident path"),
                                }
                            } else {
                                panic!("Error parsing an ident path");
                            }
                        }
                    }
                    _ => panic!("unexpected token when parsing ternary"),
                }
            }
            (value_true, value_false)
        };

        let (value_true, value_false) = parse_ternary(&mut it);

        values.push((key.clone(), value_true, value_false));

        seen_keys.push(key);
    }
    values
}

impl TrueBitflagInfo {
    fn parse(it: &mut token_stream::IntoIter) -> Self {
        let it = it.into_iter();

        let mut info: TrueBitflagInfo = Default::default();

        const REQUIRED_KEYS: &[&str] = &["name", "type", "options"];
        let mut seen_keys = Vec::new();

        loop {
            let key = match it.next() {
                Some(TokenTree::Ident(ident)) => ident.to_string(),
                Some(_) => panic!("Expected Ident or end"),
                None => break,
            };

            if seen_keys.contains(&key) {
                panic!(
                    "Duplicated key \"{}\". Keys can only be specified once.",
                    key
                );
            }

            assert_eq!(expect_punct(it), ':');

            match key.as_str() {
                "type" => info.bitflag_type = try_ident(it).expect("type: expects an ident"),
                "name" => info.bitflag_name = try_ident(it).expect("name: expects an ident"),
                "options" => info.bitflag_options = expect_ternary_group(it),
                _ => panic!(
                    "Unknown key \"{}\". Valid top level keys are: {:?}.",
                    key, REQUIRED_KEYS
                ),
            }

            assert_eq!(
                try_punct(it),
                Some(','),
                "expected comma after key-value pair {}:...",
                key.clone()
            );

            seen_keys.push(key);
        }

        expect_end(it);

        for key in REQUIRED_KEYS {
            if !seen_keys.iter().any(|e| e == key) {
                panic!("Missing required key \"{}\".", key);
            }
        }

        let mut ordered_keys: Vec<&str> = Vec::new();
        for key in REQUIRED_KEYS {
            if seen_keys.iter().any(|e| e == key) {
                ordered_keys.push(key);
            }
        }

        if seen_keys != ordered_keys {
            panic!(
                "Keys are not ordered as expected. Order them like: {:?}.",
                ordered_keys
            );
        }

        info
    }
}

/// the bitflag_options macro. Parses the token stream into a TrueBitflagInfo struct, then uses it to generate
/// the appropriate Rust code.
/// Invariant: The flags passed as input are all additive flags.
pub(crate) fn bitflag_options(ts: TokenStream) -> TokenStream {
    let mut it = ts.into_iter();
    let info = TrueBitflagInfo::parse(&mut it);
    let name = info.bitflag_name.clone();

    let try_from_inner = info
        .bitflag_options
        .iter()
        .map(|(_group_name, true_value, false_value)| {
            format!(
                "
            matched = false;

            for flag in [{true_value}, {false_value}] {{
                if (flag & to_process) == flag {{
                    matched = true;
                    to_process -= flag;
                    if flag > 0 {{
                        break;
                    }}
                }}
            }}
            if !matched {{
                return Err(value);
            }}"
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    let try_from_impl = format!(
        "impl TryFrom<<{name} as BitFlag>::Bits> for {name} {{
        type Error = <{name} as BitFlag>::Bits;

        fn try_from(value: <{name} as BitFlag>::Bits) -> Result<Self, Self::Error> {{
            let mut to_process = value.clone();

            let mut {}

            if to_process == 0 {{
                return Ok(Self(value));
            }}
            return Err(value)
        }}
    }}",
        try_from_inner
    );

    let bool_functions: String = info
        .bitflag_options
        .iter()
        .map(|(fn_name, true_value, false_value)| {
            format!(
                "
        pub fn {fn_name}(mut self, {fn_name}:bool)->Self{{
            if {fn_name}{{
                self.0 = self.0 & !{false_value} | {true_value};
            }}else{{
                self.0 = self.0 & !{true_value} | {false_value};
            }}
            self
        }}"
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    // panic!(
    //     "{:?}\n\n{}",
    //     info,
    format!(
        "
    #[derive(Debug, PartialEq)]
    pub struct {name}({type});
    impl BitFlag for {name} {{
        type Bits = {type};

        fn bits(&self) -> Self::Bits {{
            self.0
        }}
    }}

    impl {name} {{

    {bool_functions}
    }}

    impl Default for TimerMode{{
        fn default() -> Self {{
            Self(0).relative(false).pinned(false).hard(false)
        }}
    }}

    {try_from_impl}
",
        type=info.bitflag_type,
    )
    // )
    .parse()
    .expect("Error parsing formatted string into token stream.")
}
