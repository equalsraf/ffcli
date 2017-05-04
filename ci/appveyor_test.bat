echo on

cargo build --verbose --all

:: firefox can only have one active connection/session
:: disable threading in tests to avoid failures
set RUST_TEST_THREADS=1
set RUST_LOG=marionette=debug,ff=debug,manual=debug
set RUST_BACKTRACE=1

:: Some tests in the manual require a running instance,
:: Windows makes this a bit harder because we cannot write over 
:: a running .exe
set PATH=%PATH%;target\debug;
copy target\debug\ff.exe target\debug\ff-test.exe
ff-test.exe -vvv start --port 7766 || goto :error
set FF_PORT=7766

:: We dont actually run the tests for the marionette crate, just for ff
cargo test --all --verbose -- --test manual || goto :error

goto :done
:error
exit /b %errorlevel%

:done
:: Stop ff
ff-test.exe quit --port 7766
