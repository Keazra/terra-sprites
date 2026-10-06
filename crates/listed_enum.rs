// One list of variants builds the enum, its `ALL` array, and (when each
// variant stands for a fixed value) the method that returns it. Both crates
// include this file: the macro stays out of either public API, and neither
// crate takes a dependency to share it.

macro_rules! listed_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $Name:ident {
            $(
                $(#[$vmeta:meta])*
                $Variant:ident $(= $disc:literal)? => $value:expr
            ),+ $(,)?
        }
        $(#[$all_meta:meta])*
        $all_vis:vis const ALL;
        $(#[$fn_meta:meta])*
        $fn_vis:vis fn $method:ident(self) -> $ret:ty;
        $(
            $(#[$named_meta:meta])*
            $named_vis:vis fn $named:ident;
        )?
    ) => {
        listed_enum! {
            $(#[$meta])*
            $vis enum $Name {
                $(
                    $(#[$vmeta])*
                    $Variant $(= $disc)?,
                )+
            }
            $(#[$all_meta])*
            $all_vis const ALL;
        }

        impl $Name {
            $(#[$fn_meta])*
            $fn_vis fn $method(self) -> $ret {
                match self {
                    $($Name::$Variant => $value,)+
                }
            }

            $(
                $(#[$named_meta])*
                $named_vis fn $named(name: &str) -> Option<Self> {
                    Self::ALL.into_iter().find(|one| one.$method() == name)
                }
            )?
        }
    };

    (
        $(#[$meta:meta])*
        $vis:vis enum $Name:ident {
            $(
                $(#[$vmeta:meta])*
                $Variant:ident $(= $disc:literal)?
            ),+ $(,)?
        }
        $(#[$all_meta:meta])*
        $all_vis:vis const ALL;
    ) => {
        $(#[$meta])*
        $vis enum $Name {
            $(
                $(#[$vmeta])*
                $Variant $(= $disc)?,
            )+
        }

        impl $Name {
            $(#[$all_meta])*
            $all_vis const ALL: [$Name; { [$($Name::$Variant,)+].len() }] =
                [$($Name::$Variant,)+];
        }
    };
}

pub(crate) use listed_enum;
