#! /usr/bin/env python3

import sys, pywikibot

if len(sys.argv) < 4:
    raise Exception("provide format string and title and edit summary (and optionally filename)")
format_string = bytes(sys.argv[1], "utf-8").decode("unicode_escape")
title = sys.argv[2]
summary = sys.argv[3]
content = None
if len(sys.argv) >= 5:
    filename = sys.argv[4]
    with open(filename, "r") as file:
        content = file.read()

site = pywikibot.Site("en")
page = pywikibot.Page(site, title=title)
if content:
    page.text = format_string.format(content)
else:
    page.text = format_string

page.save(summary=summary, minor=False)