# UNIVAC 1219 Web Emulator

This is a version of the UNIVAC 1219 emulator adapted to run in the browser, using the Leptos web framework.

To load a `.76` file, click `Select Tape`, select your file, and click `Run`.

Similarly, assembly source can be inserted in the left assembler pane. Click `Assemble` to assemble and run the program.

Clicking `Memory` opens a memory inspector which highlights the current address of the program counter.

![Screenshot](../images/Web_Screenshot.png)

## Building
This app was based on the Leptos starter template. Consult the [Leptos book](https://book.leptos.dev/getting_started/index.html) for instructions on the necessary dependencies. Once set up, the app can be launched using

`trunk serve --open --release`

## Limitations
This app is more of an experiment, though it is able to emulate teletype behaviors (like overstriking) more accurately than the command-line emulator.