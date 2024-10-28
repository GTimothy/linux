// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2024 Google LLC.

//! A constrained Bitflag wrapper with its associated builder implementation as a macro.

#[doc(inline)]
pub use crate::macros::bitflag;

/// A trait associating a bit type to a flag type. The Flag type will be created in the macro
/// (name:<Flag> field).
pub trait BitFlag {
    /// The bit type, usually u8 or u32
    type Bits;

    /// returns the bits stored by the flag.
    fn bits(&self) -> Self::Bits;
}

/// A constrained flag's builder implements a build function once it reaches a valid state.
pub trait ConstrainedFlagBuilder<T: BitFlag> {
    /// the build function implemented by the builder once it reaches a completely valid state.
    /// This function is not implemented by the builder in any invalid state.
    fn build(self) -> T;
}

/// A marker type to use as part of the builder typestate. It indicates that part of the
/// build configuration is missing.
#[derive(Debug)]
pub struct Missing<Part> {
    t: core::marker::PhantomData<Part>,
}

/// A marker type to use as part of the builder typestate. It indicates that part of the
/// build configuration is valid.
#[derive(Debug)]
pub struct Valid<Part> {
    t: core::marker::PhantomData<Part>,
}

#[test]
#[allow(unused_variables)]
#[allow(dead_code)]
fn simple_size_colour_bitflag() {
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
    assert_eq!(flag.bits(), BIG | RED);

    let res: Result<CustomBitFlag, u32> = 1u32.try_into();
    assert_eq!(res, Err(1), "1u should fail");
    let res: Result<CustomBitFlag, u32> = (RED | BLUE).try_into();
    assert_eq!(res, Err(RED | BLUE), "RED|BLUE should fail");
    let res: Result<CustomBitFlag, u32> = (SMALL | GREEN).try_into();
    assert_eq!(
        res,
        Ok(CustomBitFlag(SMALL | GREEN)),
        "Small Green should succeed"
    );
}

#[test]
fn hrtimer_bitflag() {
    pub(crate) mod bindings {
        pub(crate) const HRTIMER_MODE_ABS: u8 = 0x00;
        pub(crate) const HRTIMER_MODE_REL: u8 = 0x01;
        pub(crate) const HRTIMER_MODE_PINNED: u8 = 0x02;
        pub(crate) const HRTIMER_MODE_SOFT: u8 = 0x04;
        pub(crate) const HRTIMER_MODE_HARD: u8 = 0x08;
    }

    use bindings::HRTIMER_MODE_HARD;

    bitflag! [
        name: TimerMode,
        type: u8,
        groups_of_incompatible: {
            AbsRel:{
            absolute: bindings::HRTIMER_MODE_ABS,
            relative: bindings::HRTIMER_MODE_REL,
            },
            Pin:{
            pinned: bindings::HRTIMER_MODE_PINNED,
            unpinned: 0, // pinned is optionnal
            },
            SoftHard:{
            soft: bindings::HRTIMER_MODE_SOFT, //with path
            hard: HRTIMER_MODE_HARD, //without path
        },
        },
    ];

    use bindings::{HRTIMER_MODE_ABS, HRTIMER_MODE_PINNED, HRTIMER_MODE_SOFT};
    let flag_builder = TimerMode::builder();
    let flag = flag_builder
        .with_absolute()
        .with_pinned()
        .with_soft()
        .build();
    assert_eq!(
        flag.bits(), // <TimerMode as BitFlag>::bits(&flag),
        bindings::HRTIMER_MODE_ABS | HRTIMER_MODE_PINNED | HRTIMER_MODE_SOFT
    );

    let res: Result<TimerMode, u8> =
        (HRTIMER_MODE_ABS | HRTIMER_MODE_PINNED | HRTIMER_MODE_SOFT | HRTIMER_MODE_HARD).try_into();
    assert_eq!(
        res,
        Err(HRTIMER_MODE_ABS | HRTIMER_MODE_PINNED | HRTIMER_MODE_SOFT | HRTIMER_MODE_HARD),
        "HRTIMER_MODE_SOFT | HRTIMER_MODE_HARD are incompatible, this should fail"
    );

    let res: Result<TimerMode, u8> = (HRTIMER_MODE_ABS).try_into();
    assert_eq!(
        res,
        Err(HRTIMER_MODE_ABS),
        "we need to specify whether pinned or not, and whether soft or hard, this should fail"
    );
}
