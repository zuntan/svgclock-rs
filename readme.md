[Japanese](readme_ja.md)

# svgclock-rs

A desktop utility program that displays a clock using SVG files.

Inspired by TzClock (https://theknight.co.uk/).

# Features

-  Displays the current time using SVG files created with Inkscape.
- Can display clocks in the following 7 designs. (Ver 0.2.0)
    - Classic Design
        - ![Without Second Hand Dial](screenshot/clock_theme_1_a.png)
        - ![With Second Hand Dial](screenshot/clock_theme_1_b.png)
    - Simple Square Design
		- ![Without second hand dial](screenshot/clock_theme_2_a.png)
        - ![With second hand dial](screenshot/clock_theme_2_b.png)
    - Modern design (includes digital display)
        -  ![Without second hand dial](screenshot/clock_theme_3.png)
	- Cool design (translucent)
        -  ![No second hand dial](screenshot/clock_theme_4.png)
    - Small green design
        -  ![No second hand dial](screenshot/clock_theme_5.png)
	- Monochrome design
        -  ![No second hand dial](screenshot/clock_theme_6.png)
    - Beach design (Looks like a painting. Image sourced from https://min-chi.material.jp/)
        -  ![No second hand dial](screenshot/clock_theme_7.png)
- You can also use SVG files created with user-generated Inkscape (https://inkscape.org/).
- This program is written in RUST (https://www.rust-lang.org/). It uses GTK-3 (https://www.gtk.org/) as its GUI library.
	- Currently provides binaries for the following platforms (multi-platform). Future support for other platforms is under consideration.
        - Windows 11
            - Video → https://www.youtube.com/watch?v=8_VTcSsL2fU
        - Ubuntu 24.04
			- Video → https://www.youtube.com/watch?v=UmCPHFl7AOQ

# Installation

Download the latest binary from the [release page](https://github.com/zuntan/svgclock-rs/releases).

- Windows
    - Extract the downloaded zip file and run the included svgclock-rs.exe.
	- The zip file includes the GTK-3 runtime library (dll).
- Ubuntu 24.04
    - Install the downloaded deb file using the following command:
        - `sudo dpkg -i svgclock-rs_0.1.0-1_amd64.deb`
	- The deb file does not include the GTK-3 runtime. If needed, install the GTK-3 runtime package separately. (It is typically already installed in Ubuntu Desktop environments.)

# Creating Clock Designs

You can create your own clock designs using Inkscape (https://inkscape.org/).

## Explanation Using a Simple Design

Open [Simple Theme](./clock_theme_custom.svg) in Inkscape.

![Simple Theme](./clock_theme_custom.svg)
- This file is also included in the package.
    - For Windows: theme/clock_theme_custom.svg in the directory where you extracted the zip file
	- For Ubuntu: /usr/share/svgclock-rs/theme/clock_theme_custom.svg

When opened in Inkscape, it will appear as follows:

![Edit](screenshot/edit_clock_theme_custom.png)

- The `base` layer contains the design for the “clock face”. You can freely design this.
- Layer `long_handle` contains the design for the “long hand” of the clock. You can design it freely. Position it to point at 12 o'clock.
- Layer `short_handle` contains the design for the “short hand” of the clock. You can design it freely. Position it to point at 12 o'clock.
- Layer `second_handle` contains the design for the “second hand” of the clock. You can design it freely. Position it to point at 12 o'clock.
- Layer `center_circle` contains the design for the “rotation center of the hands.” It must contain at least one circle or ellipse. The hour hand, minute hand, and second hand rotate around the center of this circle.
- Layer `center_circle` contains design settings specified as text. You can use the characters set in the text contained in this layer to specify the design name, etc. This layer is not drawn. Images are available at https://min-chi.material.jp/ or

## Applying Your Design to svgclock-rs

Run the program from the command line, specifying the THEME_CUSTOM environment variable.

- Windows (PowerShell)
```
$Env:THEME_CUSTOM = “clock_theme_custom.svg”; <PATH/TO/>svgclock-rs.exe
```

- Linux
```
THEME_CUSTOM=clock_theme_custom.svg <PATH/TO/>svgclock-rs```
```

Specify `<PATH/TO/>` as needed.

After launching the program, select `Preferences -> Theme -> [CUSTOM]` from the right-click menu to display your created design.

To lock the clock time for design verification, set the environment variable `FIX_TIME`.

- windows (PowerShell)
```
$Env:FIX_TIME=10:15:20; $Env:THEME_CUSTOM = “clock_theme_custom.svg”; <PATH/TO/>svgclock-rs.exe
```

- Linux
```
FIX_TIME=10:15:20 THEME_CUSTOM=clock_theme_custom.svg <PATH/TO/>svgclock-rs```
```

## Brief Description of SVG Files

ToDo.

## Design Creation Examples

Open the following SVG files in Inkscape for reference.

- ![Theme 1](./clock_theme_1.svg)
- ![Theme 2](./clock_theme_2.svg)
- ![Theme 3](./clock_theme_3.svg)
- ![Theme 4](./clock_theme_4.svg)
- ![Theme 5](./clock_theme_5.svg)
 - ![Theme 6](./clock_theme_6.svg)
- ![Theme 7](./clock_theme_7.svg)


# Acknowledgments

I would like to thank the author of TzClock (https://theknight.co.uk/) for the inspiration.

I would like to thank the image creator at “Minchirie” (https://min-chi.material.jp/) for providing the images.


Translated with DeepL.com (free version)