#! /usr/bin/env python3

import sys, pywikibot, re

if len(sys.argv) < 4:
    raise Exception("provide title and date of dump and edit summary")
title = sys.argv[1]
date = sys.argv[2]
if not re.fullmatch(r'\d{4}-\d{2}-\d{2}', date):
    raise Exception("Date '{}' should be in yyyy-mm-dd format".format(date))
summary = sys.argv[3]

site = pywikibot.Site("en")
page = pywikibot.Page(site, title=title)
page.text = re.sub(
    r"(<!-- (\{\{subst:.+?\}\}) -->).*$",
    lambda match: match.group(1) + "\n" + match.group(2).replace("...", date),
    page.text,
    flags=re.DOTALL)
page.save(summary=summary, minor=False)