#!/usr/bin/env python3
"""
Contact Reminder - a tiny "keep in touch" app.

Keeps a list of people (name, phone, email, address, birthday) and how often
you want to get in touch with each one. It reminds you when someone is due
and when a birthday is coming up.

Pure Python standard library (tkinter) - works on Windows and Linux.

Usage:
    python contacts_app.py                     open the contacts window
    python contacts_app.py --check             show a reminder popup only if
                                               someone is due (for start-up)
    python contacts_app.py --list              print who is due in the terminal
    python contacts_app.py --install-startup   run --check when you log in
    python contacts_app.py --install-shortcut  add a desktop / menu shortcut
    python contacts_app.py --uninstall         remove start-up + shortcuts
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field, fields
from datetime import date, datetime, timedelta
from pathlib import Path

APP_NAME = "Contact Reminder"
APP_ID = "contact-reminder"
APP_DIR = Path(__file__).resolve().parent
CONFIG_FILE = APP_DIR / "config.json"

DEFAULT_CONFIG = {
    # Where your contacts live. Relative paths are relative to this app folder.
    # Keep it OUTSIDE the repo, or keep the default (contacts.csv is gitignored).
    "contacts_file": "contacts.csv",
    # Warn about birthdays this many days ahead.
    "birthday_warning_days": 7,
    # While the main window is open, re-check every N hours (0 = never).
    "recheck_hours": 3,
    # Appearance (also changeable from inside the app):
    # theme: "system", "dark" or "light"
    "theme": "system",
    # font: "System" (the OS default) or "Sans"
    "font": "System",
    "font_size": 11,
}

# Offered in the Group box when adding a contact (you can type any group name).
# Override with "suggested_groups": [...] in config.json.
DEFAULT_GROUPS = ["Family", "Extended family", "Friends", "Coworkers"]
NO_GROUP = "\x00none"

# Table columns (also the keys used for sorting)
SORT_COLUMNS = ("name", "group", "phone", "birthday", "every", "last", "due", "status")

COLUMNS = [
    "name", "group", "phone", "email", "address", "birthday",
    "contact_every_days", "last_contacted", "snoozed_until", "notes",
]


# --------------------------------------------------------------------------- #
# Config + storage
# --------------------------------------------------------------------------- #

def load_config() -> dict:
    cfg = dict(DEFAULT_CONFIG)
    if CONFIG_FILE.exists():
        try:
            cfg.update(json.loads(CONFIG_FILE.read_text(encoding="utf-8")))
        except (OSError, ValueError) as exc:
            print(f"Warning: could not read {CONFIG_FILE.name}: {exc}", file=sys.stderr)
    env = os.environ.get("CONTACTS_FILE")
    if env:
        cfg["contacts_file"] = env
    return cfg


def save_settings(updates: dict) -> None:
    """Merge appearance settings into config.json (gitignored), keeping other keys."""
    data = {}
    if CONFIG_FILE.exists():
        try:
            data = json.loads(CONFIG_FILE.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            data = {}
    data.update(updates)
    try:
        CONFIG_FILE.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    except OSError as exc:
        print(f"Warning: could not save settings: {exc}", file=sys.stderr)


def contacts_path(cfg: dict) -> Path:
    p = Path(os.path.expandvars(os.path.expanduser(str(cfg["contacts_file"]))))
    if not p.is_absolute():
        p = APP_DIR / p
    return p.resolve()


def parse_date(text: str) -> date | None:
    text = (text or "").strip()
    if not text:
        return None
    for fmt in ("%Y-%m-%d", "%d/%m/%Y", "%Y/%m/%d"):
        try:
            return datetime.strptime(text, fmt).date()
        except ValueError:
            pass
    return None


def parse_birthday(text: str) -> tuple[int | None, int, int] | None:
    """Accept YYYY-MM-DD, MM-DD, --MM-DD or DD/MM/YYYY. Returns (year|None, month, day)."""
    text = (text or "").strip()
    if not text:
        return None
    d = parse_date(text)
    if d:
        return d.year, d.month, d.day
    t = text.lstrip("-")
    # Year unknown: parse against leap year 2000 so 02-29 is accepted.
    for candidate, fmt in ((f"2000-{t}", "%Y-%m-%d"), (f"{t}/2000", "%d/%m/%Y")):
        try:
            d = datetime.strptime(candidate, fmt).date()
            return None, d.month, d.day
        except ValueError:
            pass
    return None


def birthday_on(year: int, month: int, day: int) -> date:
    try:
        return date(year, month, day)
    except ValueError:          # 29 Feb in a non-leap year
        return date(year, 2, 28)


@dataclass(eq=False)   # compare by identity, so two identical rows stay separate
class Contact:
    name: str = ""
    group: str = ""
    phone: str = ""
    email: str = ""
    address: str = ""
    birthday: str = ""
    contact_every_days: str = ""
    last_contacted: str = ""
    snoozed_until: str = ""
    notes: str = ""

    # ---- derived values ------------------------------------------------- #
    @property
    def every(self) -> int:
        try:
            return max(0, int(str(self.contact_every_days).strip() or 0))
        except ValueError:
            return 0

    def next_due(self, today: date) -> date | None:
        if self.every <= 0:
            return None
        last = parse_date(self.last_contacted)
        due = last + timedelta(days=self.every) if last else today
        snooze = parse_date(self.snoozed_until)
        if snooze and snooze > due:
            due = snooze
        return due

    def days_until_due(self, today: date) -> int | None:
        due = self.next_due(today)
        return None if due is None else (due - today).days

    def next_birthday(self, today: date) -> date | None:
        b = parse_birthday(self.birthday)
        if not b:
            return None
        _, m, d = b
        nb = birthday_on(today.year, m, d)
        if nb < today:
            nb = birthday_on(today.year + 1, m, d)
        return nb

    def days_until_birthday(self, today: date) -> int | None:
        nb = self.next_birthday(today)
        return None if nb is None else (nb - today).days

    def turning_age(self, today: date) -> int | None:
        b = parse_birthday(self.birthday)
        nb = self.next_birthday(today)
        if not b or b[0] is None or nb is None:
            return None
        return nb.year - b[0]

    def status(self, today: date, bday_days: int) -> str:
        parts = []
        dd = self.days_until_due(today)
        if dd is not None:
            if dd < 0:
                parts.append(f"Overdue {-dd}d")
            elif dd == 0:
                parts.append("Due today")
            else:
                parts.append(f"In {dd}d")
        db = self.days_until_birthday(today)
        if db is not None and db <= bday_days:
            age = self.turning_age(today)
            age_txt = f" ({age})" if age else ""
            parts.append("Birthday TODAY" + age_txt if db == 0 else f"Birthday in {db}d{age_txt}")
        return " · ".join(parts)

    def needs_attention(self, today: date, bday_days: int) -> bool:
        dd = self.days_until_due(today)
        db = self.days_until_birthday(today)
        return (dd is not None and dd <= 0) or (db is not None and db <= bday_days)


class ContactStore:
    def __init__(self, path: Path):
        self.path = path
        self.contacts: list[Contact] = []

    def load(self) -> None:
        self.contacts = []
        if not self.path.exists():
            return
        with open(self.path, newline="", encoding="utf-8-sig") as fh:
            for row in csv.DictReader(fh):
                row = {(k or "").strip().lower(): (v or "").strip() for k, v in row.items()}
                if not row.get("name"):
                    continue
                self.contacts.append(Contact(**{c: row.get(c, "") for c in COLUMNS}))

    def save(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        fd, tmp = tempfile.mkstemp(prefix=".contacts-", suffix=".csv", dir=self.path.parent)
        try:
            with os.fdopen(fd, "w", newline="", encoding="utf-8-sig") as fh:
                w = csv.DictWriter(fh, fieldnames=COLUMNS)
                w.writeheader()
                for c in sorted(self.contacts, key=lambda c: c.name.lower()):
                    w.writerow({f.name: getattr(c, f.name) for f in fields(Contact)})
            os.replace(tmp, self.path)
        except BaseException:
            try:
                os.remove(tmp)
            except OSError:
                pass
            raise

    def ensure_exists(self) -> None:
        if not self.path.exists():
            self.save()

    def attention(self, today: date, bday_days: int) -> list[Contact]:
        items = [c for c in self.contacts if c.needs_attention(today, bday_days)]
        return sorted(items, key=lambda c: sort_key(c, today))


def sort_key(c: Contact, today: date):
    dd = c.days_until_due(today)
    db = c.days_until_birthday(today)
    candidates = [x for x in (dd, db) if x is not None]
    return (min(candidates) if candidates else 10**6, c.name.lower())


# --------------------------------------------------------------------------- #
# Look & feel: themes and text size
# --------------------------------------------------------------------------- #

# Soft cream-on-charcoal rather than pure white-on-black: lower glare,
# which is easier on the eyes.
THEMES = {
    "dark": {
        "bg": "#1f1f1f", "surface": "#262626", "surface2": "#303030", "border": "#3b3b3b",
        "text": "#e3dfd1", "muted": "#a19d92", "accent": "#f76f53",
        "primary": "#dcd7c6", "primary_text": "#1f1f1f", "primary_hover": "#ebe6d6",
        "hover": "#3a3a3a", "select": "#5a3328", "select_text": "#f3efe4",
        "overdue": "#47251f", "today": "#433a20", "bday": "#1f3a3f",
        "strip": "#2f4a3f",
    },
    "light": {
        "bg": "#f3efe4", "surface": "#fbf9f3", "surface2": "#e7e2d4", "border": "#d3cdbd",
        "text": "#2b2a27", "muted": "#6a665c", "accent": "#b8452d",
        "primary": "#2b2a27", "primary_text": "#f3efe4", "primary_hover": "#45433e",
        "hover": "#ddd7c7", "select": "#f0c6b9", "select_text": "#2b2a27",
        "overdue": "#f6d6cf", "today": "#f5e7c1", "bday": "#d3e9ec",
        "strip": "#2f4a3f",
    },
}

FONT_CHOICES = {
    "System": [],
    "Sans": ["Verdana", "Segoe UI", "DejaVu Sans", "Noto Sans"],
}
FALLBACK_FONTS = ["Verdana", "Segoe UI", "DejaVu Sans", "Noto Sans", "Helvetica"]


def system_prefers_dark() -> bool:
    try:
        if sys.platform.startswith("win"):
            import winreg
            key = winreg.OpenKey(winreg.HKEY_CURRENT_USER,
                                 r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
            return winreg.QueryValueEx(key, "AppsUseLightTheme")[0] == 0
        if shutil.which("gsettings"):
            out = subprocess.run(["gsettings", "get", "org.gnome.desktop.interface", "color-scheme"],
                                 capture_output=True, text=True, timeout=2)
            if "dark" in out.stdout:
                return True
            out = subprocess.run(["gsettings", "get", "org.gnome.desktop.interface", "gtk-theme"],
                                 capture_output=True, text=True, timeout=2)
            return "dark" in out.stdout.lower()
    except Exception:
        pass
    return True  # dark by default, like Zen


def windows_dark_titlebar(win, dark: bool) -> None:
    """Dark title bar on Windows 10/11 (no-op elsewhere)."""
    if not sys.platform.startswith("win"):
        return
    try:
        import ctypes
        win.update_idletasks()
        hwnd = ctypes.windll.user32.GetParent(win.winfo_id())
        value = ctypes.c_int(1 if dark else 0)
        for attr in (20, 19):  # DWMWA_USE_IMMERSIVE_DARK_MODE (new, old builds)
            if ctypes.windll.dwmapi.DwmSetWindowAttribute(
                    hwnd, attr, ctypes.byref(value), ctypes.sizeof(value)) == 0:
                break
    except Exception:
        pass


# --------------------------------------------------------------------------- #
# GUI
# --------------------------------------------------------------------------- #

def import_tk():
    try:
        import tkinter as tk
        from tkinter import font as tkfont, messagebox, ttk
        return tk, ttk, messagebox, tkfont
    except ImportError:
        sys.exit("tkinter is not installed. On Debian/Ubuntu run: sudo apt install python3-tk")


def open_in_file_manager(path: Path) -> None:
    try:
        if sys.platform.startswith("win"):
            os.startfile(str(path))  # type: ignore[attr-defined]
        elif sys.platform == "darwin":
            subprocess.Popen(["open", str(path)])
        else:
            subprocess.Popen(["xdg-open", str(path)])
    except OSError:
        pass


def desktop_notify(title: str, body: str) -> None:
    """Best-effort native notification on Linux (Windows uses the popup window)."""
    if sys.platform.startswith("linux") and shutil.which("notify-send"):
        try:
            subprocess.Popen(["notify-send", "-a", APP_NAME, title, body])
        except OSError:
            pass


class App:
    def __init__(self, store: ContactStore, cfg: dict, reminder_only: bool = False):
        self.tk, self.ttk, self.messagebox, self.tkfont = import_tk()
        self.store = store
        self.cfg = cfg
        self.bday_days = int(cfg.get("birthday_warning_days", 7))

        self.root = self.tk.Tk()
        self.root.title(APP_NAME)

        theme = str(cfg.get("theme", "system")).lower()
        if theme not in THEMES:
            theme = "dark" if system_prefers_dark() else "light"
        self.theme_name = theme
        self.font_name = cfg.get("font", "System")
        try:
            self.font_size = max(8, min(22, int(cfg.get("font_size", 11))))
        except (TypeError, ValueError):
            self.font_size = 11

        self.style = self.ttk.Style(self.root)
        self.style.theme_use("clam")  # the only built-in ttk theme that can be fully recoloured
        self.f_title = self.tkfont.Font(self.root)
        self.f_bold = self.tkfont.Font(self.root)
        self.f_small = self.tkfont.Font(self.root)
        self.f_icon = self.tkfont.Font(self.root)
        self.f_heading = self.tkfont.Font(self.root)
        self.apply_fonts()
        self.apply_theme()

        self.main_frame = None
        self.reminder_win = None
        if reminder_only:
            self.root.withdraw()
            self.show_reminder(standalone=True)
        else:
            self.build_main()
            self.schedule_recheck()

    # ---- helpers -------------------------------------------------------- #
    @property
    def today(self) -> date:
        return date.today()

    @property
    def c(self) -> dict:
        return THEMES[self.theme_name]

    def resolve_family(self) -> str:
        available = set(self.tkfont.families(self.root))
        for fam in FONT_CHOICES.get(self.font_name, []) + FALLBACK_FONTS:
            if fam in available:
                return fam
        return self.tkfont.nametofont("TkDefaultFont").actual("family")

    def apply_fonts(self):
        fam, size = self.resolve_family(), self.font_size
        for name in ("TkDefaultFont", "TkTextFont", "TkMenuFont", "TkHeadingFont",
                     "TkCaptionFont", "TkTooltipFont"):
            try:
                self.tkfont.nametofont(name).configure(family=fam, size=size)
            except self.tk.TclError:
                pass
        self.tkfont.nametofont("TkHeadingFont").configure(weight="bold")
        self.f_title.configure(family=fam, size=size + 15, weight="bold")
        self.f_bold.configure(family=fam, size=size + 1, weight="bold")
        self.f_small.configure(family=fam, size=max(8, size - 1))
        self.f_icon.configure(family=fam, size=size + 2)
        line = self.tkfont.nametofont("TkDefaultFont").metrics("linespace")
        self.style.configure("Treeview", rowheight=int(line * 1.7))  # roomy rows
        self.style.configure("TButton", width=-6)
        self.f_heading.configure(family=fam, size=size + 5, weight="bold")
        if getattr(self, "main_frame", None):
            self.fit_columns()

    def apply_theme(self):
        c, s = self.c, self.style
        self.root.configure(bg=c["bg"])
        s.configure(".", background=c["bg"], foreground=c["text"], bordercolor=c["border"],
                    lightcolor=c["bg"], darkcolor=c["bg"], troughcolor=c["surface"],
                    focuscolor=c["accent"], selectbackground=c["select"],
                    selectforeground=c["select_text"], insertcolor=c["text"])
        s.configure("TFrame", background=c["bg"])
        s.configure("Card.TFrame", background=c["surface"])
        s.configure("TLabel", background=c["bg"], foreground=c["text"])
        s.configure("Card.TLabel", background=c["surface"], foreground=c["text"])
        s.configure("Title.TLabel", font=self.f_title, foreground=c["text"])
        s.configure("Muted.TLabel", foreground=c["muted"])
        s.configure("CardMuted.TLabel", background=c["surface"], foreground=c["muted"])
        s.configure("CardBold.TLabel", background=c["surface"], foreground=c["text"], font=self.f_bold)
        s.configure("Accent.TLabel", background=c["surface"], foreground=c["accent"], font=self.f_bold)
        s.configure("SelectionCount.TLabel", background=c["bg"], foreground=c["accent"], font=self.f_bold)

        pad = (14, 7)
        s.configure("TButton", background=c["surface2"], foreground=c["text"], padding=pad,
                    bordercolor=c["border"], lightcolor=c["surface2"], darkcolor=c["surface2"],
                    relief="flat", focusthickness=1)
        s.map("TButton",
              background=[("pressed", c["border"]), ("active", c["hover"])],
              lightcolor=[("active", c["hover"])], darkcolor=[("active", c["hover"])])
        s.configure("Primary.TButton", background=c["primary"], foreground=c["primary_text"],
                    bordercolor=c["primary"], lightcolor=c["primary"], darkcolor=c["primary"],
                    padding=pad)
        s.map("Primary.TButton",
              background=[("pressed", c["primary_hover"]), ("active", c["primary_hover"])],
              lightcolor=[("active", c["primary_hover"])], darkcolor=[("active", c["primary_hover"])])
        s.configure("Icon.TButton", font=self.f_icon, padding=(10, 4))

        s.configure("TMenubutton", background=c["surface2"], foreground=c["text"], padding=pad,
                    bordercolor=c["border"], lightcolor=c["surface2"], darkcolor=c["surface2"],
                    arrowcolor=c["text"])
        s.map("TMenubutton", background=[("active", c["hover"])])

        s.configure("TEntry", fieldbackground=c["surface"], foreground=c["text"],
                    bordercolor=c["border"], lightcolor=c["surface"], darkcolor=c["surface"],
                    insertcolor=c["text"], padding=5)
        s.map("TEntry", bordercolor=[("focus", c["accent"])], lightcolor=[("focus", c["accent"])])

        s.configure("Treeview", background=c["surface"], fieldbackground=c["surface"],
                    foreground=c["text"], bordercolor=c["border"],
                    lightcolor=c["surface"], darkcolor=c["surface"])
        s.map("Treeview", background=[("selected", c["select"])],
              foreground=[("selected", c["select_text"])])
        s.configure("Treeview.Heading", background=c["surface2"], foreground=c["text"],
                    bordercolor=c["border"], lightcolor=c["surface2"], darkcolor=c["surface2"],
                    relief="flat", padding=(6, 6))
        s.map("Treeview.Heading", background=[("active", c["hover"])])

        for orient in ("Vertical", "Horizontal"):
            s.configure(f"{orient}.TScrollbar", background=c["surface2"], troughcolor=c["bg"],
                        bordercolor=c["bg"], arrowcolor=c["muted"],
                        lightcolor=c["surface2"], darkcolor=c["surface2"], gripcount=0)
            s.map(f"{orient}.TScrollbar", background=[("active", c["hover"])])
        s.configure("TSeparator", background=c["border"])

        s.configure("TCombobox", fieldbackground=c["surface"], background=c["surface2"],
                    foreground=c["text"], arrowcolor=c["text"], bordercolor=c["border"],
                    lightcolor=c["surface"], darkcolor=c["surface"], insertcolor=c["text"],
                    selectbackground=c["select"], selectforeground=c["select_text"], padding=4)
        s.map("TCombobox", fieldbackground=[("readonly", c["surface"])],
              bordercolor=[("focus", c["accent"])], background=[("active", c["hover"])])
        for opt, val in (("background", c["surface"]), ("foreground", c["text"]),
                         ("selectBackground", c["select"]), ("selectForeground", c["select_text"])):
            self.root.option_add(f"*TCombobox*Listbox.{opt}", val)

        if hasattr(self, "tree"):
            self.tree.tag_configure("overdue", background=c["overdue"], foreground=c["text"])
            self.tree.tag_configure("today", background=c["today"], foreground=c["text"])
            self.tree.tag_configure("bday", background=c["bday"], foreground=c["text"])
            self.tree.tag_configure("divider", background=c["bg"], foreground=c["accent"],
                                    font=self.f_bold)
        if hasattr(self, "theme_btn"):
            self.theme_btn.configure(text="☀" if self.theme_name == "dark" else "☾")
        self.recolor_tk(self.root)

    def recolor_tk(self, widget):
        """Plain tk widgets (windows, text boxes, menus) don't follow ttk styles."""
        c = self.c
        cls = widget.winfo_class()
        try:
            if cls in ("Tk", "Toplevel"):
                widget.configure(bg=c["bg"])
                windows_dark_titlebar(widget, self.theme_name == "dark")
            elif cls == "Text":
                widget.configure(bg=c["surface"], fg=c["text"], insertbackground=c["text"],
                                 selectbackground=c["select"], selectforeground=c["select_text"],
                                 highlightbackground=c["border"], highlightcolor=c["accent"],
                                 highlightthickness=1, relief="flat", bd=0, padx=6, pady=4,
                                 font="TkTextFont")
            elif cls == "Menu":
                widget.configure(bg=c["surface"], fg=c["text"], activebackground=c["select"],
                                 activeforeground=c["select_text"], selectcolor=c["accent"],
                                 relief="flat", bd=1)
            elif cls == "Frame" and getattr(widget, "_is_strip", False):
                widget.configure(bg=c["strip"])
        except self.tk.TclError:
            pass
        for child in widget.winfo_children():
            self.recolor_tk(child)

    def make_window(self, title: str):
        win = self.tk.Toplevel(self.root)
        win.title(title)
        self.recolor_tk(win)
        return win

    def toggle_theme(self):
        self.theme_name = "light" if self.theme_name == "dark" else "dark"
        save_settings({"theme": self.theme_name})
        self.apply_theme()

    def set_font(self, name: str | None = None, delta: int = 0):
        if name:
            self.font_name = name
        self.font_size = max(8, min(22, self.font_size + delta))
        save_settings({"font": self.font_name, "font_size": self.font_size})
        self.apply_fonts()
        self.apply_theme()

    def save(self) -> bool:
        try:
            self.store.save()
            return True
        except PermissionError:
            self.messagebox.showerror(
                APP_NAME, f"Couldn't save - is {self.store.path.name} open in another program "
                          f"(e.g. Excel)? Close it and try again.")
        except OSError as exc:
            self.messagebox.showerror(APP_NAME, f"Couldn't save contacts:\n{exc}")
        return False

    def mark_contacted(self, c: Contact):
        c.last_contacted = self.today.isoformat()
        c.snoozed_until = ""
        self.save()

    def snooze(self, c: Contact, days: int = 1):
        c.snoozed_until = (self.today + timedelta(days=days)).isoformat()
        self.save()

    def accent_strip(self, parent):
        strip = self.tk.Frame(parent, height=4, bd=0, highlightthickness=0)
        strip._is_strip = True
        strip.configure(bg=self.c["strip"])
        strip.pack(fill="x", side="top")
        return strip

    # ---- main window ---------------------------------------------------- #
    def build_main(self):
        tk, ttk = self.tk, self.ttk
        root = self.root
        root.deiconify()
        root.geometry("1180x680")
        root.minsize(860, 440)

        self.sort_col = self.cfg.get("sort", "status")
        if self.sort_col not in SORT_COLUMNS:
            self.sort_col = "status"
        self.sort_desc = bool(self.cfg.get("sort_desc", False))
        self.group_filter = ""          # "" = all groups

        self.accent_strip(root)
        frame = ttk.Frame(root, padding=(24, 16, 24, 14))
        frame.pack(fill="both", expand=True)
        self.main_frame = frame

        # Header: big title on the left, controls on the right (like Zen's nav)
        top = ttk.Frame(frame)
        top.pack(fill="x")
        ttk.Label(top, text="Contacts", style="Title.TLabel").pack(side="left")

        right = ttk.Frame(top)
        right.pack(side="right", anchor="n")
        self.theme_btn = ttk.Button(right, style="Icon.TButton", width=3, command=self.toggle_theme,
                                    text="☀" if self.theme_name == "dark" else "☾")
        self.theme_btn.pack(side="right")
        root.bind_all("<Control-d>", lambda e: self.toggle_theme())

        # Settings menu: start-up, shortcuts, update - no terminal needed
        settings_btn = ttk.Menubutton(right, text="⚙  Settings")
        self.settings_menu = tk.Menu(settings_btn, tearoff=False)
        self.startup_var = tk.BooleanVar(value=startup_installed())
        self.settings_menu.add_checkbutton(label="Remind me when I log in", variable=self.startup_var,
                                           command=self.toggle_startup)
        self.settings_menu.add_command(label="Add desktop shortcut", command=self.add_shortcut)
        self.settings_menu.add_separator()
        self.settings_menu.add_command(label="Update & restart", command=self.update_and_restart)
        self.settings_menu.add_command(label="Open app folder", command=lambda: open_in_file_manager(APP_DIR))
        settings_btn["menu"] = self.settings_menu
        settings_btn.pack(side="right", padx=(0, 8))

        font_btn = ttk.Menubutton(right, text="Aa  Font")
        self.font_menu = tk.Menu(font_btn, tearoff=False)
        self.font_var = tk.StringVar(value=self.font_name)
        for name in FONT_CHOICES:
            self.font_menu.add_radiobutton(label=name, value=name, variable=self.font_var,
                                           command=lambda n=name: self.set_font(n))
        self.font_menu.add_separator()
        self.font_menu.add_command(label="Bigger text    Ctrl +", command=lambda: self.set_font(delta=1))
        self.font_menu.add_command(label="Smaller text   Ctrl -", command=lambda: self.set_font(delta=-1))
        font_btn["menu"] = self.font_menu
        font_btn.pack(side="right", padx=(0, 8))
        for seq in ("<Control-plus>", "<Control-equal>"):
            root.bind_all(seq, lambda e: self.set_font(delta=1))
        root.bind_all("<Control-minus>", lambda e: self.set_font(delta=-1))

        sub = ttk.Frame(frame)
        sub.pack(fill="x", pady=(2, 14))
        ttk.Label(sub, text="Keep in touch with the people who matter.",
                  style="Muted.TLabel").pack(side="left")
        self.search_var = tk.StringVar()
        self.search_var.trace_add("write", lambda *_: self.refresh())
        ttk.Entry(sub, textvariable=self.search_var, width=22).pack(side="right")
        ttk.Label(sub, text="Search").pack(side="right", padx=(0, 8))

        bar = ttk.Frame(frame)
        bar.pack(fill="x", pady=(0, 10))
        ttk.Button(bar, text="+ Add", style="Primary.TButton",
                   command=self.add_contact).pack(side="left", padx=(0, 8))
        for text, cmd in [
            ("Edit", self.edit_selected),
            ("Delete", self.delete_selected),
            ("Contacted ✓", self.contacted_selected),
            ("Snooze", self.snooze_selected),
            ("Reminders", lambda: self.show_reminder(standalone=False, force=True)),
        ]:
            ttk.Button(bar, text=text, command=cmd).pack(side="left", padx=(0, 8))

        self.selection_var = tk.StringVar()
        ttk.Label(bar, textvariable=self.selection_var, style="SelectionCount.TLabel"
                  ).pack(side="left", padx=(6, 0))

        # Group filter
        self.group_btn = ttk.Menubutton(bar, text="Group: All")
        self.group_menu = tk.Menu(self.group_btn, tearoff=False)
        self.group_btn["menu"] = self.group_menu
        self.group_btn.pack(side="right")

        # footer first (side=bottom) so it never gets pushed off-screen
        foot = ttk.Frame(frame)
        foot.pack(side="bottom", fill="x", pady=(10, 0))
        self.status_var = tk.StringVar()
        ttk.Label(foot, textvariable=self.status_var, style="Muted.TLabel").pack(side="left")
        ttk.Button(foot, text="Open data folder",
                   command=lambda: open_in_file_manager(self.store.path.parent)).pack(side="right")

        cols = SORT_COLUMNS
        heads = ("Name", "Group", "Phone", "Birthday", "Every (days)", "Last contacted", "Next due", "Status")
        self._tree_heads = dict(zip(cols, heads))
        tree_wrap = ttk.Frame(frame)
        tree_wrap.pack(fill="both", expand=True)
        self.tree = ttk.Treeview(tree_wrap, columns=cols, show="headings", selectmode="extended")
        for col in cols:
            self.tree.heading(col, text=self._tree_heads[col], anchor="w",
                              command=lambda c=col: self.sort_by(c))
            self.tree.column(col, width=110, anchor="w", stretch=(col in ("name", "status")))
        sb = ttk.Scrollbar(tree_wrap, orient="vertical", command=self.tree.yview)
        hsb = ttk.Scrollbar(tree_wrap, orient="horizontal", command=self.tree.xview)
        self.tree.configure(yscrollcommand=sb.set, xscrollcommand=hsb.set)
        tree_wrap.rowconfigure(0, weight=1)
        tree_wrap.columnconfigure(0, weight=1)
        self.tree.grid(row=0, column=0, sticky="nsew")
        sb.grid(row=0, column=1, sticky="ns")
        hsb.grid(row=1, column=0, sticky="ew")
        self.tree.bind("<Double-1>", lambda e: self.edit_selected())
        self.tree.bind("<Delete>", lambda e: self.delete_selected())
        self.tree.bind("<<TreeviewSelect>>", self._skip_dividers)
        self.tree.bind("<Control-a>", self.select_all)

        self.apply_theme()
        self.refresh()

    # ---- sorting / grouping --------------------------------------------- #
    def sort_by(self, col: str):
        if col == self.sort_col:
            self.sort_desc = not self.sort_desc
        else:
            self.sort_col, self.sort_desc = col, False
        save_settings({"sort": self.sort_col, "sort_desc": self.sort_desc})
        self.refresh()

    def set_group_filter(self, group: str):
        self.group_filter = group
        self.refresh()

    def all_groups(self) -> list[str]:
        return sorted({c.group.strip() for c in self.store.contacts if c.group.strip()}, key=str.lower)

    def group_suggestions(self) -> list[str]:
        groups = self.all_groups()
        lower = {g.lower() for g in groups}
        extra = [g for g in self.cfg.get("suggested_groups", DEFAULT_GROUPS) if g.lower() not in lower]
        return groups + extra

    def rebuild_group_menu(self):
        m = self.group_menu
        m.delete(0, "end")
        self._group_var = getattr(self, "_group_var", None) or self.tk.StringVar()
        self._group_var.set(self.group_filter)
        m.add_radiobutton(label="All groups", value="", variable=self._group_var,
                          command=lambda: self.set_group_filter(""))
        groups = self.all_groups()
        if groups:
            m.add_separator()
        for g in groups:
            m.add_radiobutton(label=g, value=g, variable=self._group_var,
                              command=lambda g=g: self.set_group_filter(g))
        if any(not c.group.strip() for c in self.store.contacts):
            m.add_separator()
            m.add_radiobutton(label="No group", value=NO_GROUP, variable=self._group_var,
                              command=lambda: self.set_group_filter(NO_GROUP))
        label = "All" if not self.group_filter else ("No group" if self.group_filter == NO_GROUP
                                                     else self.group_filter)
        self.group_btn.configure(text=f"Group: {label}")
        self.recolor_tk(m)

    def sort_value(self, c: Contact, col: str, today: date):
        if col == "name":
            return c.name.lower()
        if col == "group":
            return c.group.strip().lower() or None
        if col == "phone":
            return c.phone.strip() or None
        if col == "birthday":
            return c.days_until_birthday(today)      # soonest birthday first
        if col == "every":
            return c.every or None
        if col == "last":
            return parse_date(c.last_contacted)
        if col == "due":
            return c.next_due(today)
        return None

    def sections(self, items: list[Contact], today: date) -> list[tuple[str | None, list[Contact]]]:
        """Split into titled sections (shown with divider rows) and sort each one."""
        col, desc = self.sort_col, self.sort_desc
        by_name = lambda c: c.name.lower()
        if col == "status":
            urgency = lambda c: sort_key(c, today)
            need = [c for c in items if c.needs_attention(today, self.bday_days)]
            need_ids = {id(c) for c in need}
            later = [c for c in items if id(c) not in need_ids and c.every > 0]
            done = need_ids | {id(c) for c in later}
            off = [c for c in items if id(c) not in done]
            out = [("Needs attention", sorted(need, key=urgency, reverse=desc)),
                   ("Up to date", sorted(later, key=urgency, reverse=desc)),
                   ("No reminder set", sorted(off, key=urgency, reverse=desc))]
            return [s for s in out if s[1]]
        if col == "group":
            buckets: dict[str, list[Contact]] = {}
            for c in items:
                buckets.setdefault(c.group.strip(), []).append(c)
            names = sorted((g for g in buckets if g), key=str.lower, reverse=desc)
            if "" in buckets:
                names.append("")
            return [(g or "No group", sorted(buckets[g], key=by_name)) for g in names]
        has = [c for c in items if self.sort_value(c, col, today) is not None]
        missing = [c for c in items if self.sort_value(c, col, today) is None]
        has.sort(key=lambda c: (self.sort_value(c, col, today), by_name(c)), reverse=desc)
        return [(None, has + sorted(missing, key=by_name))]   # blanks always last

    def _skip_dividers(self, _event=None):
        for iid in self.tree.selection():
            if iid not in self._row_map:
                self.tree.selection_remove(iid)
        n = len(self.tree.selection())
        if hasattr(self, "selection_var"):
            self.selection_var.set(f"{n} selected — hold Ctrl or Shift to pick more" if n == 1
                                   else (f"{n} selected" if n > 1 else ""))

    def select_all(self, _event=None):
        """Ctrl+A: select every visible contact (not the section dividers)."""
        self.tree.selection_set([i for i in self.tree.get_children() if i in self._row_map])
        return "break"

    def refresh(self):
        if not self.main_frame:
            return
        today = self.today
        q = self.search_var.get().strip().lower()
        self.tree.delete(*self.tree.get_children())
        self._row_map = {}

        items = []
        for c in self.store.contacts:
            if self.group_filter == NO_GROUP and c.group.strip():
                continue
            if self.group_filter not in ("", NO_GROUP) and c.group.strip() != self.group_filter:
                continue
            hay = " ".join([c.name, c.group, c.phone, c.email, c.address, c.notes]).lower()
            if q and q not in hay:
                continue
            items.append(c)

        for title, members in self.sections(items, today):
            if title is not None:
                self.tree.insert("", "end", values=(f"{title}  ({len(members)})",) + ("",) * 7,
                                 tags=("divider",))
            for c in members:
                dd = c.days_until_due(today)
                db = c.days_until_birthday(today)
                tag = ()
                if dd is not None and dd < 0:
                    tag = ("overdue",)
                elif dd == 0:
                    tag = ("today",)
                elif db is not None and db <= self.bday_days:
                    tag = ("bday",)
                due = c.next_due(today)
                iid = self.tree.insert("", "end", values=(
                    c.name, c.group, c.phone, c.birthday, c.contact_every_days or "-",
                    c.last_contacted or "never", due.isoformat() if due else "-",
                    c.status(today, self.bday_days)), tags=tag)
                self._row_map[iid] = c

        arrow = " ▼" if self.sort_desc else " ▲"
        for col in self.tree["columns"]:
            self.tree.heading(col, text=self._tree_heads[col] + (arrow if col == self.sort_col else ""))
        self.rebuild_group_menu()
        self.fit_columns()
        need = self.store.attention(today, self.bday_days)
        shown = f"  ·  showing {len(items)}" if len(items) != len(self.store.contacts) else ""
        self.status_var.set(f"{len(self.store.contacts)} contacts  ·  {len(need)} need attention{shown}"
                            "   ·   click a column heading to sort")

    def fit_columns(self):
        """Size each column to its widest text so wide fonts still fit."""
        body = self.tkfont.nametofont("TkDefaultFont")
        head = self.tkfont.nametofont("TkHeadingFont")
        rows = [self.tree.item(i, "values") for i in self._row_map]
        for idx, col in enumerate(self.tree["columns"]):
            w = head.measure(self._tree_heads[col] + " ▼")
            for r in rows:
                w = max(w, body.measure(str(r[idx])))
            if idx == 0:   # section dividers live in the first column, in bold
                for i in self.tree.get_children():
                    if i not in self._row_map:
                        w = max(w, self.f_bold.measure(self.tree.item(i, "values")[0]))
            self.tree.column(col, width=w + 28, minwidth=min(w + 28, 260))

    # ---- settings menu actions ------------------------------------------ #
    def toggle_startup(self):
        try:
            if self.startup_var.get():
                where = install_startup()
                self.messagebox.showinfo(APP_NAME, f"Reminders will pop up when you log in.\n\n{where}")
            else:
                remove_startup()
                self.messagebox.showinfo(APP_NAME, "Start-up reminder removed.")
        except Exception as exc:
            self.messagebox.showerror(APP_NAME, f"Couldn't change start-up setting:\n{exc}")
        self.startup_var.set(startup_installed())

    def add_shortcut(self):
        try:
            where = install_shortcut()
            self.messagebox.showinfo(APP_NAME, "Shortcut added:\n\n" + "\n".join(where))
        except Exception as exc:
            self.messagebox.showerror(APP_NAME, f"Couldn't add shortcut:\n{exc}")

    def update_and_restart(self):
        """Pull new code (if this is a git checkout), refresh shortcuts, then restart."""
        notes = []
        if (APP_DIR / ".git").exists() and shutil.which("git"):
            out = run_quiet(["git", "-C", str(APP_DIR), "pull", "--ff-only"])
            text = (out.stdout + out.stderr).strip()
            if out.returncode != 0:
                if not self.messagebox.askyesno(
                        APP_NAME, f"Couldn't download the update:\n\n{text}\n\nRestart anyway?"):
                    return
            else:
                notes.append(text.splitlines()[-1] if text else "Up to date.")
        # Make sure the new code is valid before closing the old version
        try:
            import py_compile
            py_compile.compile(str(APP_DIR / "contacts_app.py"), doraise=True)
        except py_compile.PyCompileError as exc:
            self.messagebox.showerror(APP_NAME, "The updated app has an error, so I'm not restarting.\n\n"
                                                f"{exc.msg}")
            return
        # Re-point existing shortcuts at this folder (handy if the folder was moved)
        try:
            if startup_installed():
                install_startup()
            if shortcut_installed():
                install_shortcut()
        except Exception as exc:
            notes.append(f"(Shortcuts not refreshed: {exc})")
        if notes:
            print("\n".join(notes))
        relaunch()
        self.root.destroy()

    def selected_list(self) -> list[Contact]:
        """All selected contacts, in the order shown (ignores divider rows)."""
        sel = [i for i in self.tree.selection() if i in self._row_map]
        if not sel:
            self.messagebox.showinfo(APP_NAME, "Select one or more contacts first "
                                                "(click one, or Ctrl/Shift-click for more).")
            return []
        return [self._row_map[i] for i in sel]

    def add_contact(self):
        c = Contact(contact_every_days="30")
        if self.edit_dialog(c, new=True):
            self.store.contacts.append(c)
            self.save()
            self.refresh()

    def edit_selected(self):
        items = self.selected_list()
        if not items:
            return
        if len(items) == 1:
            changed = self.edit_dialog(items[0])
        else:
            changed = self.batch_edit_dialog(items)
        if changed:
            self.save()
            self.refresh()

    def delete_selected(self):
        items = self.selected_list()
        if not items:
            return
        if len(items) == 1:
            msg = f"Delete {items[0].name}?"
        else:
            preview = ", ".join(c.name for c in items[:10])
            if len(items) > 10:
                preview += f", + {len(items) - 10} more"
            msg = f"Delete these {len(items)} contacts?\n\n{preview}"
        if self.messagebox.askyesno(APP_NAME, msg):
            for c in items:
                self.store.contacts.remove(c)
            self.save()
            self.refresh()

    def contacted_selected(self):
        items = self.selected_list()
        if not items:
            return
        today_iso = self.today.isoformat()
        for c in items:
            c.last_contacted = today_iso
            c.snoozed_until = ""
        self.save()
        self.refresh()

    def snooze_selected(self):
        items = self.selected_list()
        if not items:
            return
        until = (self.today + timedelta(days=1)).isoformat()
        for c in items:
            c.snoozed_until = until
        self.save()
        self.refresh()

    def edit_dialog(self, c: Contact, new: bool = False) -> bool:
        tk, ttk = self.tk, self.ttk
        win = self.make_window("Add contact" if new else f"Edit {c.name}")
        win.transient(self.root)
        win.resizable(True, False)
        self.accent_strip(win)
        body = ttk.Frame(win, padding=18)
        body.pack(fill="both", expand=True)
        body.columnconfigure(1, weight=1)

        labels = [
            ("name", "Name *", ""),
            ("group", "Group", "Pick one or type a new group"),
            ("phone", "Phone", ""),
            ("email", "Email", ""),
            ("address", "Address", ""),
            ("birthday", "Birthday", "YYYY-MM-DD, or MM-DD if year unknown"),
            ("contact_every_days", "Contact every (days)", "0 or blank = no reminder"),
            ("last_contacted", "Last contacted", "YYYY-MM-DD"),
            ("notes", "Notes", ""),
        ]
        vars_ = {}
        widgets = {}
        for r, (key, label, hint) in enumerate(labels):
            ttk.Label(body, text=label).grid(row=r * 2, column=0, sticky="nw", padx=(0, 12), pady=(6, 0))
            if key in ("address", "notes"):
                w = tk.Text(body, height=3, width=44, wrap="word")
                w.insert("1.0", getattr(c, key))
            elif key == "group":
                vars_[key] = tk.StringVar(value=c.group or (self.group_filter
                                                            if self.group_filter not in ("", NO_GROUP) else ""))
                w = ttk.Combobox(body, textvariable=vars_[key], values=self.group_suggestions(), width=44)
            else:
                vars_[key] = tk.StringVar(value=getattr(c, key))
                w = ttk.Entry(body, textvariable=vars_[key], width=46)
            w.grid(row=r * 2, column=1, sticky="ew", pady=(6, 0))
            widgets[key] = w
            if hint:
                ttk.Label(body, text=hint, style="Muted.TLabel", font=self.f_small
                          ).grid(row=r * 2 + 1, column=1, sticky="w")
        if not new:
            ttk.Button(body, text="Today",
                       command=lambda: vars_["last_contacted"].set(date.today().isoformat())
                       ).grid(row=[k for k, *_ in labels].index("last_contacted") * 2,
                              column=2, padx=(8, 0), pady=(6, 0))
        self.recolor_tk(win)

        result = {"ok": False}

        def get(key):
            w = widgets[key]
            return w.get("1.0", "end").strip() if isinstance(w, tk.Text) else vars_[key].get().strip()

        def on_save():
            name = get("name")
            if not name:
                self.messagebox.showwarning(APP_NAME, "Name is required.", parent=win)
                return
            bday = get("birthday")
            if bday and not parse_birthday(bday):
                self.messagebox.showwarning(APP_NAME, "Birthday should look like 1990-05-17 or 05-17.", parent=win)
                return
            last = get("last_contacted")
            if last and not parse_date(last):
                self.messagebox.showwarning(APP_NAME, "Last contacted should look like 2026-01-31.", parent=win)
                return
            every = get("contact_every_days")
            if every and not every.isdigit():
                self.messagebox.showwarning(APP_NAME, "Contact every should be a whole number of days.", parent=win)
                return
            for key, *_ in labels:
                setattr(c, key, get(key))
            result["ok"] = True
            win.destroy()

        btns = ttk.Frame(body)
        btns.grid(row=len(labels) * 2, column=0, columnspan=3, sticky="e", pady=(16, 0))
        ttk.Button(btns, text="Cancel", command=win.destroy).pack(side="right")
        ttk.Button(btns, text="Save", style="Primary.TButton", command=on_save).pack(side="right", padx=(0, 8))
        win.bind("<Return>", lambda e: on_save() if not isinstance(e.widget, tk.Text) else None)
        win.bind("<Escape>", lambda e: win.destroy())
        widgets["name"].focus_set()
        win.grab_set()
        self.root.wait_window(win)
        return result["ok"]

    def batch_edit_dialog(self, contacts: list[Contact]) -> bool:
        """Change one or more fields on several contacts at once. Only ticked fields change."""
        tk, ttk = self.tk, self.ttk
        win = self.make_window(f"Edit {len(contacts)} contacts")
        win.transient(self.root)
        win.resizable(True, False)
        self.accent_strip(win)
        body = ttk.Frame(win, padding=18)
        body.pack(fill="both", expand=True)
        body.columnconfigure(1, weight=1)

        preview = ", ".join(c.name for c in contacts[:8])
        if len(contacts) > 8:
            preview += f", + {len(contacts) - 8} more"
        ttk.Label(body, text=f"Editing {len(contacts)} contacts:", font=self.f_bold
                  ).grid(row=0, column=0, columnspan=3, sticky="w")
        ttk.Label(body, text=preview, style="Muted.TLabel", wraplength=440, justify="left"
                  ).grid(row=1, column=0, columnspan=3, sticky="w", pady=(2, 16))
        ttk.Label(body, text="Tick a field to apply it to all of them. Untouched fields are left alone.",
                  style="Muted.TLabel", font=self.f_small, wraplength=440, justify="left"
                  ).grid(row=2, column=0, columnspan=3, sticky="w", pady=(0, 14))

        r = 3
        use_group = tk.BooleanVar(value=False)
        group_var = tk.StringVar()
        ttk.Checkbutton(body, variable=use_group, text="Set group to").grid(row=r, column=0, sticky="w", pady=5)
        ttk.Combobox(body, textvariable=group_var, values=self.group_suggestions(), width=28
                     ).grid(row=r, column=1, sticky="ew", padx=(8, 0))
        r += 1
        ttk.Label(body, text="Leave blank to remove the group", style="Muted.TLabel", font=self.f_small
                  ).grid(row=r, column=1, sticky="w", padx=(8, 0))
        r += 1

        use_every = tk.BooleanVar(value=False)
        every_var = tk.StringVar()
        ttk.Checkbutton(body, variable=use_every, text="Set contact every (days) to"
                        ).grid(row=r, column=0, sticky="w", pady=5)
        ttk.Entry(body, textvariable=every_var, width=10).grid(row=r, column=1, sticky="w", padx=(8, 0))
        r += 1
        ttk.Label(body, text="0 or blank = no reminder", style="Muted.TLabel", font=self.f_small
                  ).grid(row=r, column=1, sticky="w", padx=(8, 0))
        r += 1

        mark_contacted = tk.BooleanVar(value=False)
        ttk.Checkbutton(
            body, variable=mark_contacted,
            text=f"Mark all as contacted today ({self.today.isoformat()})"
        ).grid(row=r, column=0, columnspan=2, sticky="w", pady=(10, 5))
        r += 1

        clear_snooze = tk.BooleanVar(value=False)
        ttk.Checkbutton(body, variable=clear_snooze, text="Clear any snooze on these contacts"
                        ).grid(row=r, column=0, columnspan=2, sticky="w", pady=5)
        r += 1

        self.recolor_tk(win)
        result = {"ok": False}

        def on_apply():
            every_txt = every_var.get().strip()
            if use_every.get() and every_txt and not every_txt.isdigit():
                self.messagebox.showwarning(APP_NAME, "Contact every should be a whole number of days.",
                                            parent=win)
                return
            if not any((use_group.get(), use_every.get(), mark_contacted.get(), clear_snooze.get())):
                self.messagebox.showinfo(APP_NAME, "Tick at least one field to change.", parent=win)
                return
            today_iso = self.today.isoformat()
            group_val = group_var.get().strip()
            for c in contacts:
                if use_group.get():
                    c.group = group_val
                if use_every.get():
                    c.contact_every_days = every_txt
                if mark_contacted.get():
                    c.last_contacted = today_iso
                    c.snoozed_until = ""
                elif clear_snooze.get():
                    c.snoozed_until = ""
            result["ok"] = True
            win.destroy()

        btns = ttk.Frame(body)
        btns.grid(row=r, column=0, columnspan=3, sticky="e", pady=(18, 0))
        ttk.Button(btns, text="Cancel", command=win.destroy).pack(side="right")
        ttk.Button(btns, text=f"Apply to {len(contacts)}", style="Primary.TButton",
                   command=on_apply).pack(side="right", padx=(0, 8))
        win.bind("<Escape>", lambda e: win.destroy())
        win.grab_set()
        self.root.wait_window(win)
        return result["ok"]

    # ---- reminder popup ------------------------------------------------- #
    def show_reminder(self, standalone: bool, force: bool = False):
        ttk = self.ttk
        today = self.today
        items = self.store.attention(today, self.bday_days)
        if not items:
            if standalone:
                self.root.destroy()
            elif force:
                self.messagebox.showinfo(APP_NAME, "Nobody is due right now.")
            return
        if self.reminder_win is not None and self.reminder_win.winfo_exists():
            self.reminder_win.destroy()

        desktop_notify(APP_NAME, f"{len(items)} people to get in touch with")
        win = self.make_window(f"{APP_NAME} - time to reach out")
        self.reminder_win = win
        win.attributes("-topmost", True)
        win.after(1500, lambda: win.winfo_exists() and win.attributes("-topmost", False))
        self.accent_strip(win)
        outer = ttk.Frame(win, padding=18)
        outer.pack(fill="both", expand=True)
        ttk.Label(outer, text="Time to reach out", font=self.f_heading).pack(anchor="w")
        ttk.Label(outer, text=today.strftime("%A %d %B %Y"),
                  style="Muted.TLabel").pack(anchor="w", pady=(0, 12))

        list_frame = ttk.Frame(outer, style="Card.TFrame", padding=(14, 6))
        list_frame.pack(fill="both", expand=True)

        def render():
            for w in list_frame.winfo_children():
                w.destroy()
            current = self.store.attention(self.today, self.bday_days)
            if not current:
                ttk.Label(list_frame, text="All caught up ✓", style="Card.TLabel").pack(anchor="w", pady=8)
                return
            for i, c in enumerate(current):
                if i:
                    ttk.Separator(list_frame).pack(fill="x")
                row = ttk.Frame(list_frame, style="Card.TFrame", padding=(0, 8))
                row.pack(fill="x")
                info = ttk.Frame(row, style="Card.TFrame")
                info.pack(side="left", fill="x", expand=True)
                # Zen-changelog style: coloured status word, then the name
                line = ttk.Frame(info, style="Card.TFrame")
                line.pack(anchor="w")
                ttk.Label(line, text=c.status(self.today, self.bday_days),
                          style="Accent.TLabel").pack(side="left", padx=(0, 10))
                ttk.Label(line, text=c.name, style="CardBold.TLabel").pack(side="left")
                contact_bits = "  ·  ".join(x for x in (c.phone, c.email) if x)
                if contact_bits:
                    ttk.Label(info, text=contact_bits, style="CardMuted.TLabel").pack(anchor="w")
                ttk.Button(row, text="Tomorrow",
                           command=lambda c=c: (self.snooze(c, 1), render(), self.refresh())
                           ).pack(side="right", padx=(8, 0))
                if c.every > 0:
                    ttk.Button(row, text="Done ✓", style="Primary.TButton",
                               command=lambda c=c: (self.mark_contacted(c), render(), self.refresh())
                               ).pack(side="right")

        render()

        foot = ttk.Frame(outer)
        foot.pack(fill="x", pady=(14, 0))

        def open_main():
            win.destroy()
            if not self.main_frame:
                self.build_main()
                self.schedule_recheck()
            self.root.deiconify()
            self.root.lift()

        def close():
            win.destroy()
            if not self.main_frame:
                self.root.destroy()

        ttk.Button(foot, text="Open contacts", command=open_main).pack(side="left")
        ttk.Button(foot, text="Close", command=close).pack(side="right")
        win.protocol("WM_DELETE_WINDOW", close)
        win.bind("<Escape>", lambda e: close())
        win.update_idletasks()
        win.minsize(520, 0)

    def schedule_recheck(self):
        hours = float(self.cfg.get("recheck_hours", 0) or 0)
        if hours <= 0:
            return
        ms = int(hours * 3600 * 1000)

        def tick():
            try:
                self.store.load()      # pick up edits made elsewhere (e.g. Excel)
            except OSError:
                pass
            self.refresh()
            if self.store.attention(self.today, self.bday_days):
                self.show_reminder(standalone=False)
            self.root.after(ms, tick)

        self.root.after(ms, tick)

    def run(self):
        self.root.mainloop()


# --------------------------------------------------------------------------- #
# Start-up / shortcut install (Windows + Linux)
# --------------------------------------------------------------------------- #

def gui_python() -> str:
    exe = Path(sys.executable)
    if sys.platform.startswith("win"):
        pyw = exe.with_name("pythonw.exe")
        if pyw.exists():
            return str(pyw)
    return str(exe)


def run_quiet(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    """Run a helper command without flashing a console window on Windows."""
    if sys.platform.startswith("win"):
        kw.setdefault("creationflags", 0x08000000)  # CREATE_NO_WINDOW
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def _ps_quote(s: str) -> str:
    return "'" + s.replace("'", "''") + "'"


def win_shortcut(folder_key: str, lnk_name: str, args: str, remove: bool = False) -> str:
    """Create/remove a .lnk in a Windows special folder ('Startup', 'Desktop', 'Programs')."""
    script = APP_DIR / "contacts_app.py"
    ps = (
        "$ws = New-Object -ComObject WScript.Shell;"
        f"$p = Join-Path $ws.SpecialFolders({_ps_quote(folder_key)}) {_ps_quote(lnk_name + '.lnk')};"
    )
    if remove:
        ps += "if (Test-Path $p) { Remove-Item $p }; Write-Output $p"
    else:
        ps += (
            "$s = $ws.CreateShortcut($p);"
            f"$s.TargetPath = {_ps_quote(gui_python())};"
            f"$s.Arguments = {_ps_quote(chr(34) + str(script) + chr(34) + (' ' + args if args else ''))};"
            f"$s.WorkingDirectory = {_ps_quote(str(APP_DIR))};"
            f"$s.Description = {_ps_quote(APP_NAME)};"
            "$s.Save(); Write-Output $p"
        )
    out = run_quiet(["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps])
    if out.returncode != 0:
        raise RuntimeError(out.stderr.strip() or "PowerShell failed")
    return out.stdout.strip()


def linux_desktop_entry(args: str, extra: str = "") -> str:
    script = APP_DIR / "contacts_app.py"
    return (
        "[Desktop Entry]\n"
        "Type=Application\n"
        f"Name={APP_NAME}\n"
        "Comment=Remind me to keep in touch with people\n"
        f'Exec="{gui_python()}" "{script}"{(" " + args) if args else ""}\n'
        "Icon=contact-new\n"
        "Terminal=false\n"
        "Categories=Utility;\n"
        f"{extra}"
    )


def linux_desktop_dir() -> Path:
    if shutil.which("xdg-user-dir"):
        try:
            out = subprocess.run(["xdg-user-dir", "DESKTOP"], capture_output=True, text=True)
            if out.returncode == 0 and out.stdout.strip():
                return Path(out.stdout.strip())
        except OSError:
            pass
    return Path.home() / "Desktop"


def linux_write(path: Path, content: str) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    path.chmod(0o755)
    if shutil.which("gio"):  # lets GNOME launch it without the "untrusted" prompt
        subprocess.run(["gio", "set", str(path), "metadata::trusted", "true"],
                       capture_output=True)
    return str(path)


def _startup_file() -> Path:
    if sys.platform.startswith("win"):
        return (Path(os.environ.get("APPDATA", Path.home() / "AppData/Roaming"))
                / "Microsoft/Windows/Start Menu/Programs/Startup" / f"{APP_NAME}.lnk")
    return Path.home() / ".config/autostart" / f"{APP_ID}.desktop"


def _menu_file() -> Path:
    if sys.platform.startswith("win"):
        return (Path(os.environ.get("APPDATA", Path.home() / "AppData/Roaming"))
                / "Microsoft/Windows/Start Menu/Programs" / f"{APP_NAME}.lnk")
    return Path.home() / ".local/share/applications" / f"{APP_ID}.desktop"


def startup_installed() -> bool:
    return _startup_file().exists()


def shortcut_installed() -> bool:
    return _menu_file().exists()


def install_startup() -> str:
    if sys.platform.startswith("win"):
        return win_shortcut("Startup", APP_NAME, "--check")
    return linux_write(_startup_file(),
                       linux_desktop_entry("--check", "X-GNOME-Autostart-enabled=true\n"))


def remove_startup() -> None:
    if sys.platform.startswith("win"):
        win_shortcut("Startup", APP_NAME, "", remove=True)
    elif _startup_file().exists():
        _startup_file().unlink()


def install_shortcut() -> list[str]:
    if sys.platform.startswith("win"):
        return [win_shortcut("Desktop", APP_NAME, ""), win_shortcut("Programs", APP_NAME, "")]
    entry = linux_desktop_entry("")
    made = [linux_write(_menu_file(), entry)]
    desk = linux_desktop_dir()
    if desk.is_dir():
        made.append(linux_write(desk / f"{APP_ID}.desktop", entry))
    return made


def uninstall() -> None:
    if sys.platform.startswith("win"):
        for key in ("Startup", "Desktop", "Programs"):
            print("Removed:", win_shortcut(key, APP_NAME, "", remove=True))
    else:
        for p in (_startup_file(), _menu_file(), linux_desktop_dir() / f"{APP_ID}.desktop"):
            if p.exists():
                p.unlink()
                print("Removed:", p)
    print("Your contacts file was not touched.")


def relaunch(*args: str) -> None:
    """Start a fresh copy of the app (used by Update & restart)."""
    cmd = [gui_python(), str(APP_DIR / "contacts_app.py"), *args]
    if sys.platform.startswith("win"):
        flags = 0x00000008 | 0x00000200   # DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
        subprocess.Popen(cmd, cwd=str(APP_DIR), creationflags=flags, close_fds=True)
    else:
        subprocess.Popen(cmd, cwd=str(APP_DIR), start_new_session=True, close_fds=True)


# --------------------------------------------------------------------------- #
# Entry point
# --------------------------------------------------------------------------- #

def print_list(store: ContactStore, bday_days: int) -> None:
    today = date.today()
    items = store.attention(today, bday_days)
    if not items:
        print("Nobody is due right now.")
        return
    for c in items:
        bits = " · ".join(x for x in (c.phone, c.email) if x)
        print(f"- {c.name}: {c.status(today, bday_days)}" + (f"  ({bits})" if bits else ""))


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=APP_NAME)
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--check", action="store_true", help="popup only if someone is due (use at start-up)")
    g.add_argument("--list", action="store_true", help="print who is due and exit")
    g.add_argument("--install-startup", action="store_true", help="run --check when you log in")
    g.add_argument("--install-shortcut", action="store_true", help="add desktop + menu shortcuts")
    g.add_argument("--uninstall", action="store_true", help="remove start-up entry and shortcuts")
    args = ap.parse_args(argv)

    if args.install_startup:
        print(f"Start-up reminder installed: {install_startup()}")
        return 0
    if args.install_shortcut:
        for p in install_shortcut():
            print(f"Shortcut: {p}")
        return 0
    if args.uninstall:
        uninstall()
        return 0

    cfg = load_config()
    store = ContactStore(contacts_path(cfg))
    store.load()

    if args.list:
        print_list(store, int(cfg.get("birthday_warning_days", 7)))
        return 0

    if args.check:
        if not store.attention(date.today(), int(cfg.get("birthday_warning_days", 7))):
            return 0  # nothing due: exit quietly, no window
        App(store, cfg, reminder_only=True).run()
        return 0

    try:
        store.ensure_exists()
    except OSError as exc:
        print(f"Could not create {store.path}: {exc}", file=sys.stderr)
    App(store, cfg).run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
