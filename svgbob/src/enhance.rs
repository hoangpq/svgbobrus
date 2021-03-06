use focus_char::FocusChar;
use fragments::Fragment;
use location::Location;
use location::Direction::{Bottom, BottomLeft, BottomRight, Left, Right, Top, TopLeft, TopRight};
use block::Block::{A, C, E, F, J, K, M, O, P, S, U, W, Y};
use point_block::PointBlock;
use fragments::{line, arc, arrow_line};

pub trait Enhance {
    fn enhance(&self) -> (Vec<Fragment>, Vec<Location>);
}

impl<'g> Enhance for FocusChar<'g> {

    fn enhance(&self) -> (Vec<Fragment>, Vec<Location>) {
        let mut elm = vec![];
        let mut consumed = vec![];
        let a = &PointBlock::block(A);
        //let _b = &PointBlock::block(B);
        let c = &PointBlock::block(C);
        //let _d = &PointBlock::block(D);
        let e = &PointBlock::block(E);
        let f = &PointBlock::block(F);
        //let _g = &PointBlock::block(G);
        //let _h = &PointBlock::block(H);
        //let _i = &PointBlock::block(I);
        let j = &PointBlock::block(J);
        let k = &PointBlock::block(K);
        //let _l = &PointBlock::block(L);
        let m = &PointBlock::block(M);
        //let _n = &PointBlock::block(N);
        let o = &PointBlock::block(O);
        let p = &PointBlock::block(P);
        //let _q = &PointBlock::block(Q);
        //let _r = &PointBlock::block(R);
        let s = &PointBlock::block(S);
        //let _t = &PointBlock::block(T);
        let u = &PointBlock::block(U);
        //let _v = &PointBlock::block(V);
        let w = &PointBlock::block(W);
        //let _x = &PointBlock::block(X);
        let y = &PointBlock::block(Y);

        let this = || Location::this();
        let top = || Location::go(Top);
        let bottom = || Location::go(Bottom);
        let left = || Location::go(Left);
        let right = || Location::go(Right);
        let top_left = || Location::go(TopLeft);
        let top_right = || Location::go(TopRight);
        let bottom_left = || Location::go(BottomLeft);
        let bottom_right = || Location::go(BottomRight);

        let top_left2 = || top().go_left(2);
        let top_right2 = || top().go_right(2);
        let bottom_right2 = || bottom().go_right(2); 
        let bottom_left2 = || bottom().go_left(2); 


        // _ underscore
        if self.is('_') {
            //   _|
            if self.right().any("|[") {
                elm.push(line(u, &right().w()));
            }
            //    |_
            if self.left().any("|]") {
                elm.push(line(y, &left().w()));
            }
        }
        if self.any("`'") {
            // for circuitries
            //  +     +    \
            //   `>    '>   `>
            if self.top_left().any("+\\") && self.right().is('>') {
                elm.push(arrow_line(&top_left().m(), &right().f()));
                consumed.extend(vec![this(), right()]);
            }
            // for circuitries
            //     +    /
            //   <'   <'
            if self.top_right().any("+/") && self.left().is('<') {
                elm.push(arrow_line(&top_right().m(), &left().j()));
                consumed.extend(vec![this(), left()]);
            }
            // For diamond rectangle
            //     .
            //    '
            if self.top_right().any(".,") {
                elm.push(line(c, &top_right().m()));
                consumed.extend(vec![this(), top_right()]);
            }
            //   .
            //    '
            if self.top_left().any(".,") {
                elm.push(line(c, &top_left().m()));
                consumed.extend(vec![this(), top_left()]);
            }
            //   .'
            if self.left().any(".,") {
                elm.push(line(c, &left().m()));
                consumed.extend(vec![this(),left()]);
            }
            //   '.
            if self.right().any(".,") {
                elm.push(line(c, &right().m()));
                consumed.extend(vec![this(),right()]);
            }
        }
        if self.any(".,") {
            // for circuitries
            //   <.    <,   <.
            //     +     \    \
            if self.bottom_right().any("+\\") && self.left().is('<') {
                elm.push(arrow_line(&bottom_right().m(), &left().t()));
                consumed.extend(vec![this(),left()]);
            }
            // for circuitries
            //   .>    ,>   ,>
            //  +     +    /
            if self.bottom_left().any("+/") && self.right().is('>') {
                elm.push(arrow_line(&bottom_left().m(), &right().p()));
                consumed.extend(vec![this(),right()]);
            }
        }
        // transistor complimentary enhancement
        if self.is('|') {
            //    |    |
            //    <    >
            if self.bottom().any("><") {
                elm.push(line(c, &bottom().m()));
                consumed.push(this());
            }
            //    <    >
            //    |    |
            if self.top().any("><") {
                elm.push(line(w, &top().m()));
                consumed.push(this());
            }
            //    _
            //   |
            if self.top_right().is('_') {
                elm.extend(vec![line(c,w),line(c, e)]);
                consumed.push(this());
            }
            //    _
            //     |
            if self.top_left().is('_') {
                elm.extend(vec![line(c,w),line(a,c)]);
                consumed.push(this());
            }
        } 
        if self.is('/') {
            //      >
            //     /
            if self.top_right().is('>') {
                elm.push(line(u, &top_right().m()));
                consumed.push(this());
            }
            //    /
            //   <
            if self.bottom_left().is('<') {
                elm.push(line(e, &bottom_left().m()));
                consumed.push(this());
            }
        } 
        if self.is('\\') {
            //      \
            //       >
            if self.bottom_right().is('>') {
                elm.push(line(a, &bottom_right().m()));
                consumed.push(this());
            }
            //    <
            //     \
            if self.top_left().is('<') {
                elm.push(line(y, &top_left().m()));
                consumed.push(this());
            }
        }

        if self.any("vV"){
            //     `.
            //       V
            if self.top_left().is('.') && self.top().in_left(2).is('`'){
                elm.push(arrow_line(&top_left2().c(), j));
                consumed.push(this())
            }
            //    .'
            //   V
            if self.top_right().is('.') && self.top().in_right(2).is('\''){
                elm.push(arrow_line(&top_right2().c(), f));
                consumed.push(this())
            }
        }
        if self.is('^'){
            //  ^
            //   `.
            if self.bottom_right().is('`') && self.bottom().in_right(2).is('.'){
                elm.push(arrow_line(&bottom_right2().t(), m));
                consumed.push(this());
            }
            //    ^
            //  .'
            if self.bottom_left().is('\'') && self.bottom().in_left(2).is('.') {
                elm.push(arrow_line(&bottom_left2().p(), m));
                consumed.push(this());
            }
        }
        // circuitries jump
        //    |
        //   -(-
        //    |
        //
       if self.is('(') && self.top().can_strongly_connect(&W)
            && self.bottom().can_strongly_connect(&C)
            && self.left().can_strongly_connect(&O)
            && self.right().can_strongly_connect(&K)
        {
            elm.extend(vec![arc(c, w, 5), line(k, o)]);
            consumed.push(this());
        }
        // circuitries jump
        //    |
        //   -)-
        //    |
        //
        if self.is(')') && self.top().can_strongly_connect(&W)
            && self.bottom().can_strongly_connect(&C)
            && self.left().can_strongly_connect(&O)
            && self.right().can_strongly_connect(&K)
        {
            elm.extend(vec![arc(w, c, 5), line(k, o)]);
            consumed.push(this());
        }

        (elm, consumed)
    }
}
