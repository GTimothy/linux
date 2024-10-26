// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2024 Google LLC.

//! A constrained Bitflag wrapper with its associated builder implementation as a macro.

#[doc(inline)]
pub use crate::macros::bitflag;

/// A marker type to use as part of the BitflagBuilder typestate. It indicates that part of the build configuration is missing.
#[derive(Debug)]
pub struct Missing<Part> {
    t: core::marker::PhantomData<Part>,
}

/// A marker type to use as part of the BitflagBuilder typestate. It indicates that part of the build configuration is valid.
#[derive(Debug)]
pub struct Valid<Part> {
    t: core::marker::PhantomData<Part>,
}

#[test]
#[allow(unused_variables)]
#[allow(dead_code)]
fn simple_size_colour_bitflag() {
    use crate::macros::bitflag;

    const BIG: u32 = 0u32;
    const SMALL: u32 = 1u32;

    const RED: u32 = 2u32;
    const GREEN: u32 = 4u32;
    const BLUE: u32 = 7u32;

    bitflag! [
        name: CustomBitFlag,
        type: u32,
        groups_of_incompatible: {
            Size:{
                big: BIG,
                small: SMALL,
            },
            Colour:{
                red:RED,
                green:GREEN,
                blue:BLUE,
            },
        },
    ];

    let flag_builder = CustomBitFlag::builder();
    let flag = flag_builder.with_big().with_red().build();
    assert_eq!(flag.0, BIG | RED);
}
