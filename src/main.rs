extern crate libc;
use libc::c_char;
use libc::c_int;
use libc::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::ptr;

#[repr(C)]
struct deno_buf {
    data: *const c_char,
    len: c_int, // TODO(ry) should be size_t.
}

#[repr(C)]
struct Deno {
    _unused: [u8; 0],
}

type DenoSubCb = extern "C" fn(d: *mut Deno, channel: *const c_char, buf: deno_buf);

#[link(name = "deno", kind = "static")]
extern "C" {
    fn deno_init();
    fn deno_v8_version() -> *const c_char;
    fn deno_set_flags(argc: *mut c_int, argv: *mut *mut c_char);

    fn deno_new(data: *const c_void, cb: DenoSubCb) -> *const Deno;
    fn deno_delete(d: *const Deno);
    fn deno_last_exception(d: *const Deno) -> *const c_char;
    fn deno_set_response(d: *const Deno, buf: deno_buf);
    fn deno_execute(d: *const Deno, js_filename: *const c_char, js_source: *const c_char) -> c_int;
}

// Pass the command line arguments to v8.
// Returns a vector of command line arguments that v8 did not understand.
fn set_flags() -> Vec<String> {
    // deno_set_flags(int* argc, char** argv) mutates argc and argv to remove
    // flags that v8 understands.
    // Convert command line arguments to a vector of C strings.
    let mut argv = std::env::args()
        .map(|arg| CString::new(arg).unwrap().into_bytes_with_nul())
        .collect::<Vec<_>>();
    // Make a new array, that can be modified by V8::SetFlagsFromCommandLine(),
    // containing mutable raw pointers to the individual command line args.
    let mut c_argv = argv.iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut i8)
        .collect::<Vec<_>>();
    // Store the length of the argv array in a local variable. We'll pass a
    // pointer to this local variable to deno_set_flags(), which then
    // updates its value.
    let mut c_argc = argv.len() as c_int;
    // Let v8 parse the arguments it recognizes and remove them from c_argv.
    unsafe {
        deno_set_flags(&mut c_argc, c_argv.as_mut_ptr());
    };
    // If c_argc was updated we have to change the length of c_argv to match.
    c_argv.truncate(c_argc as usize);
    // Copy the modified arguments list into a proper rust vec and return it.
    c_argv
        .iter()
        .map(|ptr| unsafe {
            let cstr = CStr::from_ptr(*ptr as *const i8);
            let slice = cstr.to_str().unwrap();
            slice.to_string()
        })
        .collect::<Vec<_>>()
}

extern "C" fn on_message(_d: *mut Deno, _channel: *const c_char, _buf: deno_buf) {
    println!("got message in rust");
}

// wraps deno_execute
fn execute(d: *const Deno, js_filename: &str, js_source: &str) -> bool {
    let filename = CString::new(js_filename).unwrap();
    let source = CString::new(js_source).unwrap();
    let r = unsafe { deno_execute(d, filename.as_ptr(), source.as_ptr()) };
    r != 0
}

fn main() {
    let args = set_flags();
    unsafe { deno_init() };

    /*
    let v = unsafe { deno_v8_version() };
    let c_str = unsafe { CStr::from_ptr(v) };
    let version = c_str.to_str().unwrap();
    println!("version: {}", version);
    */

    let d = unsafe { deno_new(ptr::null(), on_message) };

    let ok = execute(d, "deno_main.js", "denoMain();");
    if !ok {
        let err = unsafe { deno_last_exception(d) };
        println!("Error!\n");
        std::process::exit(1);
    }

    unsafe { deno_delete(d) }
}
