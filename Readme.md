# Clipboard

This is a tool which sychronizes Clipboards between Windows-Hosts via a shared directory.

## Setup
For Usage get the latest release version and put a config.ini next to the .exe file.
The example.config.ini can be used as guidance but has to be renamed.

## Usage
Quick explanations of the three configuration lines in the config.ini:

- local_name: this is the pc-name for this local machine which the .exe is running on
- remote_names: are all pc-names which should be synchronized with this machine.
- dir_name: this is a path to the shared folder which is used for the .tmp-files

## Important
- The program doesn't register any keystrokes anymore?
  The global-horkey hook of the win-api doesn't allow non-admin-applications to read keystrokes while a admin-app is in the foreground.
  To fix this clipboard has to be run as admin to work. <a href="https://obsproject.com/forum/threads/global-hotkeys-for-other-programs-dont-work-when-obs-is-focused.160876/"> See here </a>

## Other
- Icons: <a href="https://www.freepik.com/icon/document_680057#fromView=search&page=1&position=57&uuid=332f1881-cfc5-4753-bad7-6f6a241a5bbe">Icon by Good Ware</a>
