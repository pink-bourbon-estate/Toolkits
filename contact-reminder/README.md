# Contact Reminder

A tiny desktop app that helps you keep in touch with people.

- Store **name, phone, email, address, birthday** and **"contact every X days"** for each person
- When someone is due, or a birthday is coming up, a small reminder window pops up
- Click **Done ✓** after you've messaged them, or **Tomorrow** to snooze
- Runs at start-up, and you can add a desktop shortcut
- Works on **Windows and Linux**. Pure Python standard library, nothing to `pip install`

## Groups & sorting

- Give each person a **group** (Family, Extended family, Friends, Coworkers, or type your own). Use the **Group** menu to show just one group.
- **Click any column heading to sort.** Click it again to switch between ascending ▲ and descending ▼. Your last sort is remembered.
- Sorting by **Status** (the default) splits the list into *Needs attention*, *Up to date* and *No reminder set*, with a divider between each. Sorting by **Group** puts a divider above each group.

## Selecting and editing several contacts at once

- Click one row, then **Ctrl-click** or **Shift-click** to add more, or press **Ctrl+A** to select everyone currently shown.
- **Contacted ✓** and **Snooze** apply to every selected person.
- **Delete** asks for confirmation once and removes everyone selected.
- **Edit** opens a normal single-person form when one person is selected. With several selected, it opens a **batch edit** box instead: tick only the fields you want to change (group, contact-every-days, mark as contacted today, clear snooze) and they're applied to everyone selected — anything left unticked is untouched.

## Settings menu (no terminal needed)

- **Remind me when I log in**: turns the start-up reminder on or off.
- **Add desktop shortcut**: adds a desktop and Start menu / app menu shortcut.
- **Update & restart**: reloads the app with the newest files. If the folder is a git clone, it runs `git pull` first. It also re-points your shortcuts if you've moved the folder.

## Look & feel

- **Dark / light mode**: click the ☀ / ☾ button in the top-right (or press `Ctrl+D`). The first run follows your system setting. Colours are a soft cream on charcoal with a coral accent, instead of harsh pure white on black.
- **Text**: use the **Aa Font** menu to pick the system font or a plain sans-serif, and `Ctrl +` / `Ctrl -` to change the text size.
- Your choices are saved in `config.json`.

Your contacts are stored in a plain CSV file, so you can also open and edit them in Excel or LibreOffice.

## Requirements

- Python 3.9 or newer
- tkinter, which ships with Python on Windows. On Debian/Ubuntu: `sudo apt install python3-tk`

## Quick start

```bash
python contacts_app.py                     # open the contacts window
python contacts_app.py --install-startup   # show reminders when you log in
python contacts_app.py --install-shortcut  # desktop + start menu / app menu shortcut
```

On Windows you can use `py` instead of `python`.

Other commands:

```bash
python contacts_app.py --check      # reminder popup only if someone is due (used at start-up)
python contacts_app.py --list       # print who is due in the terminal
python contacts_app.py --uninstall  # remove start-up entry and shortcuts (keeps your data)
```

## How reminders work

| Field | Meaning |
|---|---|
| `group` | Any label you like, e.g. Family or Friends. |
| `contact_every_days` | How often you want to be in touch. Blank or `0` means no reminder. |
| `last_contacted` | Set automatically when you click **Done ✓** or **Contacted today**. |
| `birthday` | `YYYY-MM-DD`, or `MM-DD` if you don't know the year. |
| `snoozed_until` | Set by **Tomorrow** / **Snooze**. |

A person is **due** when `last_contacted + contact_every_days` is today or earlier (or they've never been contacted). Birthdays show up `birthday_warning_days` ahead (7 by default).

At start-up, `--check` exits silently if nobody is due, so you only see a window when there's something to do. While the main window is open it re-checks every few hours.

## Keeping your data private

Your real contacts **never** need to be in the repo:

- `contacts.csv` and `config.json` are in `.gitignore`.
- To store the file somewhere else (e.g. a synced cloud folder), copy `config.example.json` to `config.json` and set `contacts_file` to an absolute or relative path, e.g. `"../contacts.csv"`.
- Or set the `CONTACTS_FILE` environment variable.

`contacts.example.csv` contains fictional sample data only. To try it:
`CONTACTS_FILE=contacts.example.csv python contacts_app.py --list`

## License

Code: MIT.
