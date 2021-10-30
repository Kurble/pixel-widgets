#[doc = include_str!("../declarative-syntax.md")]
#[macro_export]
macro_rules! view {
    { $w1:ident $([$($m1:ident = $v1:expr),+])? $(=>$c1:tt)? } => {
        Option::unwrap(view!{ inner $w1 $([$($m1 = $v1),+])? $(=>$c1)? })
    };

    {
        inner $widget:ident
            $([$($modifier:ident = $value:expr),+])?
            $(=>{$(
                $(:for $x:pat in $i:expr =>)?
                $(:if $(let $y:pat =)? $yc:expr =>)?
                $w1:ident $([$($m1:ident = $v1:expr),+])? $(=>$c1:tt)?
                $(:else if $(let $z:pat =)? $zc:expr => $w2:ident $([$($m2:ident = $v2:expr),+])? $(=>$c2:tt)?)*
                $(:else => $w3:ident $([$($m3:ident = $v3:expr),+])? $(=>$c3:tt)?)?
            ),+})?
    } => {
        Some($widget::default()
            $($(.extend(view!{
                inner
                $(:for $x in $i =>)?
                $(:if $(let $y =)? $yc =>)?
                $w1 $([$($m1 = $v1),+])? $(=>$c1)?
                $(:else if $(let $z =)? $zc => $w2 $([$($m2=$v2),+])? $(=>$c2)?)*
                $(:else => $w3 $([$($m3=$v3),+])? $(=>$c3)?)*
            }))+)?
            $($(.$modifier($value))+)?
            .into_node()
        )
    };

    {
        inner :for $x:pat in $i:expr => $widget:ident
            $([$($modifier:ident = $value:expr),+])?
            $(=>$content:tt)?
    } => {
        $i.into_iter().flat_map(|$x| view!{ inner $widget $([$($modifier = $value),+])? $(=>$content)?})
    };
    {
        inner :if $(let $x:pat =)? $xc:expr => $w1:ident
            $([$($m1:ident = $v1:expr),+])?
            $(=>$c1:tt)?
        $(:else if $($y:pat =)? $yc:expr => $w2:ident
            $([$($m2:ident = $v2:expr),+])?
            $(=>$c2:tt)?)*
    } => {
        if $(let $x =)? $xc {
            view!{ inner $w1 $([$($m1 = $v1),+])? $(=>$c1)?}
        }
        $(else if $(let $y =)? $yc {
            view!{ inner $w2 $([$($m2 = $v2),+])? $(=>$c2)?}
        })*
        else {
            None
        }
    };
    {
        inner :if $(let $x:pat =)? $xc:expr => $w1:ident
            $([$($m1:ident = $v1:expr),+])?
            $(=>$c1:tt)?
        $(:else if $($y:pat =)? $yc:expr => $w2:ident
            $([$($m2:ident = $v2:expr),+])?
            $(=>$c2:tt)?)*
        :else => $w3:ident
            $([$($m3:ident = $v3:expr),+])?
            $(=>$c3:tt)?
    } => {
        if $(let $x =)? $xc {
            view!{ inner $w1 $([$($m1 = $v1),+])? $(=>$c1)?}
        }
        $(else if $(let $y =)? $yc {
            view!{ inner $w2 $([$($m2 = $v2),+])? $(=>$c2)?}
        })*
        else {
            view!{ inner $w3 $([$($m3 = $v3),+])? $(=>$c3)?}
        }
    };
}
