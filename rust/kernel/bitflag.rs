// SPDX-License-Identifier: GPL-2.0

//! A constrained Bitflag wrapper with its associated builder implementation as a macro.

#[doc(inline)]
pub use crate::macros::bitflag;
pub use crate::macros::bitflag_options;

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

macro_rules! bitflag_options_rules {{

    name:$name:ident,
    type:$t:ty,
    options:[
        $({name:$fn_name:ident, true: $true_bitval:expr, false: $false_bitval:expr}),+
]} => {

        #[derive(Debug, PartialEq)]
        pub struct $name($t);

        impl BitFlag for $name {
        type Bits = $t;

        fn bits(&self) -> Self::Bits {
            self.0
        }
    }

    impl $name {
    $(pub fn $fn_name(mut self, $fn_name:bool)->Self{
            if $fn_name{
                self.0 = self.0 & !$false_bitval | $true_bitval;
            }else{
                self.0 = self.0 & !$true_bitval | $false_bitval;
            }
            self
        })+


    }

    impl Default for $name{
        fn default() -> Self {
            Self(0)$(.$fn_name(false))+
        }
    }

    impl TryFrom<<$name as BitFlag>::Bits> for $name {
        type Error = <$name as BitFlag>::Bits;

        fn try_from(value: <$name as BitFlag>::Bits) -> Result<Self, Self::Error> {
            let mut to_process = value.clone();

            let mut $(
            matched = false;

            for flag in [$true_bitval, $false_bitval] {
                if (flag & to_process) == flag {
                    matched = true;
                    to_process -= flag;
                    if flag > 0 {
                        break;
                    }
                }
            }
            if !matched {
                return Err(value);
            })+


            if to_process == 0 {
                return Ok(Self(value));
            }
            return Err(value)
        }
    }

    };
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
    use bindings::hrtimer_mode_HRTIMER_MODE_HARD;

    bitflag! [
        name: TimerMode,
        type: u32,
        groups_of_incompatible: {
            AbsRel:{
            absolute: bindings::hrtimer_mode_HRTIMER_MODE_ABS,
            relative: bindings::hrtimer_mode_HRTIMER_MODE_REL,
            },
            Pin:{
            pinned: bindings::hrtimer_mode_HRTIMER_MODE_PINNED,
            unpinned: 0, // pinned is optionnal
            },
            SoftHard:{
            soft: bindings::hrtimer_mode_HRTIMER_MODE_SOFT, //with path
            hard: hrtimer_mode_HRTIMER_MODE_HARD, //without path
        },
        },
    ];

    use bindings::{
        hrtimer_mode_HRTIMER_MODE_ABS, hrtimer_mode_HRTIMER_MODE_PINNED,
        hrtimer_mode_HRTIMER_MODE_SOFT,
    };
    let flag_builder = TimerMode::builder();
    let flag = flag_builder
        .with_absolute()
        .with_pinned()
        .with_soft()
        .build();
    assert_eq!(
        flag.bits(), // <TimerMode as BitFlag>::bits(&flag),
        bindings::hrtimer_mode_HRTIMER_MODE_ABS
            | hrtimer_mode_HRTIMER_MODE_PINNED
            | hrtimer_mode_HRTIMER_MODE_SOFT
    );

    let res: Result<TimerMode, u32> = (hrtimer_mode_HRTIMER_MODE_ABS
        | hrtimer_mode_HRTIMER_MODE_PINNED
        | hrtimer_mode_HRTIMER_MODE_SOFT
        | hrtimer_mode_HRTIMER_MODE_HARD)
        .try_into();
    assert_eq!(
        res,
        Err(hrtimer_mode_HRTIMER_MODE_ABS | hrtimer_mode_HRTIMER_MODE_PINNED | hrtimer_mode_HRTIMER_MODE_SOFT | hrtimer_mode_HRTIMER_MODE_HARD),
        "hrtimer_mode_HRTIMER_MODE_SOFT | hrtimer_mode_HRTIMER_MODE_HARD are incompatible, this should fail"
    );

    let res: Result<TimerMode, u32> = (hrtimer_mode_HRTIMER_MODE_ABS).try_into();
    assert_eq!(
        res,
        Err(hrtimer_mode_HRTIMER_MODE_ABS),
        "we need to specify whether pinned or not, and whether soft or hard, this should fail"
    );
}

#[test]
fn hrtimer_bitflag_options() {
    use bindings::hrtimer_mode_HRTIMER_MODE_HARD;

    bitflag_options! [
    name: TimerMode,
    type: u32,
    options: {
            relative: bindings::hrtimer_mode_HRTIMER_MODE_REL : bindings::hrtimer_mode_HRTIMER_MODE_ABS,
            pinned: bindings::hrtimer_mode_HRTIMER_MODE_PINNED : 0,
            hard: hrtimer_mode_HRTIMER_MODE_HARD : bindings::hrtimer_mode_HRTIMER_MODE_SOFT,
    },
    ];

    use bindings::{
        hrtimer_mode_HRTIMER_MODE_ABS, hrtimer_mode_HRTIMER_MODE_PINNED,
        hrtimer_mode_HRTIMER_MODE_REL, hrtimer_mode_HRTIMER_MODE_SOFT,
    };
    let tmode: TimerMode = TimerMode::try_from(bindings::hrtimer_mode_HRTIMER_MODE_SOFT)
        .expect("other settings have 0bit modes so they should all match");

    assert_eq!(tmode, TimerMode::default().hard(false));
    assert_eq!(tmode, TimerMode::default());

    assert_eq!(
        TimerMode::default().hard(true).pinned(true).relative(true),
        TimerMode::try_from(
            hrtimer_mode_HRTIMER_MODE_HARD
                | hrtimer_mode_HRTIMER_MODE_PINNED
                | hrtimer_mode_HRTIMER_MODE_REL
        )
        .expect("this is a valid combination of flags")
    );

    assert_eq!(
        TimerMode::default()
            .hard(false)
            .pinned(true)
            .relative(false),
        TimerMode::try_from(
            hrtimer_mode_HRTIMER_MODE_SOFT
                | hrtimer_mode_HRTIMER_MODE_PINNED
                | hrtimer_mode_HRTIMER_MODE_ABS
        )
        .expect("this is a valid combination of flags")
    );

    let flag = TimerMode::default().pinned(true).hard(false);
    assert_eq!(
        flag.bits(), // <TimerMode as BitFlag>::bits(&flag),
        bindings::hrtimer_mode_HRTIMER_MODE_ABS
            | hrtimer_mode_HRTIMER_MODE_PINNED
            | hrtimer_mode_HRTIMER_MODE_SOFT
    );

    let res: Result<TimerMode, u32> = (hrtimer_mode_HRTIMER_MODE_ABS
        | hrtimer_mode_HRTIMER_MODE_PINNED
        | hrtimer_mode_HRTIMER_MODE_SOFT
        | hrtimer_mode_HRTIMER_MODE_HARD)
        .try_into();
    assert_eq!(
        res,
        Err(hrtimer_mode_HRTIMER_MODE_ABS | hrtimer_mode_HRTIMER_MODE_PINNED | hrtimer_mode_HRTIMER_MODE_SOFT | hrtimer_mode_HRTIMER_MODE_HARD),
        "hrtimer_mode_HRTIMER_MODE_SOFT | hrtimer_mode_HRTIMER_MODE_HARD are incompatible, this should fail"
    );

    let res: Result<TimerMode, u32> = (hrtimer_mode_HRTIMER_MODE_ABS).try_into();
    assert_eq!(
        res,
        Err(hrtimer_mode_HRTIMER_MODE_ABS),
        "Although unpinned matches because it has a flag of 0, we still need to specify whether whether soft or hard"
    );
}
