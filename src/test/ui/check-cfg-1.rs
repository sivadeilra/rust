// error-pattern: ``
// compile-flags: --cfg-check

#[cfg(windows)] // should work
fn do_windows_stuff() {}

#[cfg(widnows)] // should FAIL; intentional misspelling
fn do_windows_stuff() {}

#[cfg(feature = "foo")] // should work
fn use_foo() {}

#[cfg(feature = "bar")] // should work
fn use_bar() {}

#[cfg(feature = "zebra")] // should FAIL
fn use_zebra() {}

fn test_cfg_macro() {
    cfg!(windows); // should work
    cfg!(widnows); // should fail
    cfg!(feature = "foo"); // should work
    cfg!(feature = "bar"); // should work
    cfg!(feature = "zebra"); // should FAIL
    cfg!(xxx = "foo"); // should FAIL
    cfg!(xxx); // should FAIL
    cfg!(any(windows, xxx)); // TODO: make this work!! short-circuiting is defeating this
    cfg!(any(xxx, windows)); // should FAIL
    cfg!(any(feature = "bad", windows)); // should FAIL
}

fn bad_syntactic_forms() {
    cfg!(xx = yy);
    cfg!(xx(yy));
}

