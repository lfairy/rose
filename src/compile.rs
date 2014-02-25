//! State machine compiler.

use parse;
use parse::{Expr, Greedy, NonGreedy};
use vm::{Program, Inst, Jump, Range};


/// Compile an AST into a `Program`.
pub fn compile(e: &Expr) -> Program {
    let mut p = ~[];
    compile_expr(&mut p, e);
    p
}


/// Extension methods that simplify the compiler.
trait CompileExts {
    fn push_jump(&mut self);
    fn jumps<'a>(&'a mut self, index: uint) -> &'a mut ~[uint];
}

impl CompileExts for ~[Inst] {
    fn push_jump(&mut self) {
        self.push(Jump(~[]));
    }

    fn jumps<'a>(&'a mut self, index: uint) -> &'a mut ~[uint] {
        match self[index] {
            Jump(ref mut exits) => exits,
            _ => fail!("something bad happened; it's really bad")
        }
    }
}


macro_rules! record(
    () => (
        p.len()
    );
    ($list:expr) => (
        $list.push(p.len())
    )
)


fn compile_expr(p: &mut Program, e: &Expr) {
    match *e {
        parse::Empty => (),
        parse::Range(lo, hi) => p.push(Range(lo, hi)),
        parse::Concatenate(ref inners) => {
            // Execute all children, one after the other
            for inner in inners.iter() {
                compile_expr(p, inner);
            }
        },
        parse::Alternate(ref inners) => {
            let fork = record!(); p.push_jump();

            let mut heads = ~[];
            let mut tails = ~[];
            for (i, inner) in inners.iter().enumerate() {
                record!(heads); compile_expr(p, inner);
                if i != inners.len() - 1 {
                    record!(tails); p.push_jump();
                }
            }

            p.jumps(fork).push_all_move(heads);

            let end = p.len();
            for tail in tails.move_iter() {
                p.jumps(tail).push(end);
            }
        },
        parse::Repeat(ref inner, min, max, greedy) => compile_repeat(p, *inner, min, max, greedy),
        parse::Capture(..) => fail!("captures not implemented yet")
    }
}


fn compile_repeat(p: &mut Program, inner: &Expr, min: u32, max: Option<u32>, greedy: Greedy) {
    match (min, max) {
        (_, Some(max_)) => {
            // Compile `min` repetitions
            for _ in range(0, min) {
                compile_expr(p, inner);
            }

            // Compile `max - min` optional repetitions
            let mut forks = ~[];
            for _ in range(min, max_) {
                record!(forks); p.push_jump();
                compile_expr(p, inner);
            }

            let end = p.len();
            for fork in forks.move_iter() {
                draw_fork(p.jumps(fork), 1+fork, end, greedy);
            }
        },
        (0, None) => {
            let fork = record!();
            compile_repeat(p, inner, 1, None, greedy);
            let end = p.len();
            draw_fork(p.jumps(fork), 1+fork, end, greedy);
        },
        (_, None) => {
            for _ in range(0, min-1) {
                compile_expr(p, inner);
            }

            // Draw a loop around the last repetition
            let start = record!();
            compile_expr(p, inner);
            let loopy = record!(); p.push_jump();
            draw_fork(p.jumps(loopy), start, 1+loopy, greedy);
        }
    }
}


fn draw_fork(jumps: &mut ~[uint], persist: uint, escape: uint, greedy: Greedy) {
    match greedy {
        NonGreedy => { jumps.push(escape); jumps.push(persist); },
        Greedy    => { jumps.push(persist); jumps.push(escape); }
    }
}
