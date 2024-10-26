// SPDX-License-Identifier: GPL-2.0

use crate::helpers::*;
use proc_macro::{token_stream, Delimiter, TokenStream, TokenTree};

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

        assert_eq!(expect_punct(&mut it), ':');

        let value = expect_incompat_group(&mut it);
        values.push((key.clone(), value));

        assert_eq!(expect_punct(&mut it), ',');
        seen_keys.push(key);
    }
    values
}

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

        assert_eq!(expect_punct(&mut it), ':');

        let value = try_ident(&mut it)
            .unwrap_or_else(|| panic!("flag value for flag \"{}\": Expected Ident or end", key));
        values.push((key.clone(), value));

        assert_eq!(expect_punct(&mut it), ',');
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

            assert_eq!(expect_punct(it), ',');

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
    // panic!(
    //     "{:?}\n\n{}",
    //     info,
    format!(
        "
    #[derive(Debug)]
    pub struct {name};
    impl BitFlag for {name} {{
        type Bits = {type};
    }}

    impl crate::bitflag::ConstrainedFlag::<{name}> {{
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


    impl crate::bitflag::ConstrainedFlagBuilder<{name}> for {name}Builder<{valid_generics}> {{
        fn build(self) -> crate::bitflag::ConstrainedFlag<{name}> {{
            crate::bitflag::ConstrainedFlag::<{name}>(self.flags.iter().flatten().sum::<{type}>())
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
