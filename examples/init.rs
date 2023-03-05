use std::ffi::{c_char, CString};

use once_cell::sync::OnceCell;

use ona::sys::*;

static OP_HAS_RUN: OnceCell<()> = OnceCell::new();

extern "C" fn op(_term: Term) -> Feedback {
    OP_HAS_RUN.set(()).unwrap();

    println!("Running operation!");

    Feedback {
        subs: Substitution {
            map: [Term {
                hashed: false,
                hash: 0,
                atoms: [0; 64],
            }; 28],
            success: true,
        },
        failed: false,
    }
}

trait StrExt {
    fn to_c(&self) -> *mut c_char;
}

impl StrExt for str {
    fn to_c(&self) -> *mut c_char {
        CString::new(self).unwrap().into_raw()
    }
}

fn main() {
    unsafe {
        NAR_INIT();

        NAR_AddOperation("^op".to_c(), Some(op));
        NAR_AddInputNarsese("<(a &/ ^op) =/> g>.".to_c());
        NAR_AddInputNarsese("a. :|:".to_c());
        NAR_AddInputNarsese("g! :|:".to_c());

        assert!(
            OP_HAS_RUN.get().is_some(),
            "The operation has not run like it should have."
        );
    }
}
