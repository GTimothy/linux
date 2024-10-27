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
}

/// A BitFlag's bits' wrapper with build constraints.
#[derive(Debug, PartialEq)]
pub struct ConstrainedFlag<T: BitFlag>(T::Bits);

/// A constrained flag's builder implements a build function once it reaches a valid state.
pub trait ConstrainedFlagBuilder<T: BitFlag> {
    /// the build function implemented by the builder once it reaches a completely valid state.
    /// This function is not implemented by the builder in any invalid state.
    fn build(self) -> ConstrainedFlag<T>;
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
    use crate::bitflag::{bitflag, ConstrainedFlag};

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

    let flag_builder = ConstrainedFlag::<CustomBitFlag>::builder();
    let flag = flag_builder.with_big().with_red().build();
    assert_eq!(flag.0, BIG | RED);
}

#[test]
fn test2() {
    use crate::bitflag::{bitflag, ConstrainedFlag};
    const BIG: u32 = 0u32;
    const SMALL: u32 = 1u32;

    const RED: u32 = 2u32;
    const GREEN: u32 = 4u32;
    const BLUE: u32 = 7u32;

    #[derive(Debug, PartialEq)]
    pub struct CustomBitFlag;
    impl BitFlag for CustomBitFlag {
        type Bits = u32;
    }

    impl crate::bitflag::ConstrainedFlag<CustomBitFlag> {
        pub fn builder(
        ) -> CustomBitFlagBuilder<crate::bitflag::Missing<Size>, crate::bitflag::Missing<Colour>>
        {
            CustomBitFlagBuilder {
                flags: Default::default(),
                t: core::marker::PhantomData,
            }
        }
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct CustomBitFlagBuilder<S0, S1> {
        flags: [Option<u32>; 2],
        t: core::marker::PhantomData<(S0, S1)>,
    }

    #[derive(Debug)]
    pub struct Size;

    #[derive(Debug)]
    pub struct Colour;

    impl<S1> CustomBitFlagBuilder<crate::bitflag::Valid<Size>, S1> {
        pub fn set_big(&mut self) {
            self.flags[0] = Some(BIG);
        }

        pub fn set_small(&mut self) {
            self.flags[0] = Some(SMALL);
        }
    }

    impl<S0> CustomBitFlagBuilder<S0, crate::bitflag::Valid<Colour>> {
        pub fn set_red(&mut self) {
            self.flags[1] = Some(RED);
        }

        pub fn set_green(&mut self) {
            self.flags[1] = Some(GREEN);
        }

        pub fn set_blue(&mut self) {
            self.flags[1] = Some(BLUE);
        }
    }

    impl<S1> CustomBitFlagBuilder<crate::bitflag::Missing<Size>, S1> {
        pub fn with_big(self) -> CustomBitFlagBuilder<crate::bitflag::Valid<Size>, S1> {
            let mut b: CustomBitFlagBuilder<crate::bitflag::Valid<Size>, S1> =
                unsafe { core::mem::transmute(self) };
            b.set_big();
            b
        }

        pub fn with_small(self) -> CustomBitFlagBuilder<crate::bitflag::Valid<Size>, S1> {
            let mut b: CustomBitFlagBuilder<crate::bitflag::Valid<Size>, S1> =
                unsafe { core::mem::transmute(self) };
            b.set_small();
            b
        }
    }

    impl<S0> CustomBitFlagBuilder<S0, crate::bitflag::Missing<Colour>> {
        pub fn with_red(self) -> CustomBitFlagBuilder<S0, crate::bitflag::Valid<Colour>> {
            let mut b: CustomBitFlagBuilder<S0, crate::bitflag::Valid<Colour>> =
                unsafe { core::mem::transmute(self) };
            b.set_red();
            b
        }

        pub fn with_green(self) -> CustomBitFlagBuilder<S0, crate::bitflag::Valid<Colour>> {
            let mut b: CustomBitFlagBuilder<S0, crate::bitflag::Valid<Colour>> =
                unsafe { core::mem::transmute(self) };
            b.set_green();
            b
        }

        pub fn with_blue(self) -> CustomBitFlagBuilder<S0, crate::bitflag::Valid<Colour>> {
            let mut b: CustomBitFlagBuilder<S0, crate::bitflag::Valid<Colour>> =
                unsafe { core::mem::transmute(self) };
            b.set_blue();
            b
        }
    }

    impl crate::bitflag::ConstrainedFlagBuilder<CustomBitFlag>
        for CustomBitFlagBuilder<crate::bitflag::Valid<Size>, crate::bitflag::Valid<Colour>>
    {
        fn build(self) -> crate::bitflag::ConstrainedFlag<CustomBitFlag> {
            crate::bitflag::ConstrainedFlag::<CustomBitFlag>(
                self.flags.iter().flatten().sum::<u32>(),
            )
        }
    }

    impl TryFrom<<CustomBitFlag as BitFlag>::Bits> for ConstrainedFlag<CustomBitFlag> {
        type Error = u32;

        fn try_from(value: <CustomBitFlag as BitFlag>::Bits) -> Result<Self, Self::Error> {
            let builder = Self::builder();
            let mut to_process: u32 = value.clone();

            let mut matched = false;

            for flag in [BIG, SMALL] {
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
            }
            matched = false;

            for flag in [RED, BLUE, GREEN] {
                if (flag & to_process) == flag {
                    to_process -= flag;
                    matched = true;
                    if flag > 0 {
                        break;
                    }
                }
            }
            if matched && to_process == 0 {
                return Ok(Self(value));
            }

            // Error("algorithm could not deduce valid flag combination.")
            Err(value)
        }
    }

    ConstrainedFlag::<CustomBitFlag>::builder()
        .with_big()
        .with_red()
        .build();

    let res: Result<ConstrainedFlag<CustomBitFlag>, u32> = 1u32.try_into();
    assert_eq!(res, Err(1), "1u should fail");
    let res: Result<ConstrainedFlag<CustomBitFlag>, u32> = (RED | BLUE).try_into();
    assert_eq!(res, Err(RED | BLUE), "RED BLUE should fail");
    let res: Result<ConstrainedFlag<CustomBitFlag>, u32> = (SMALL | GREEN).try_into();
    assert_eq!(
        res,
        Ok(ConstrainedFlag(SMALL | GREEN)),
        "Small Green should succeed"
    );
}
