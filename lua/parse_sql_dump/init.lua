local M = {}

local make_iter = require "parse_sql_dump.utils".make_iter
M.NULL = require "parse_sql_dump.utils".NULL

-- It would be neater to generate these by parsing the description of the table in the SQL file.
M.babel = make_iter("babel", "babel", [[
user user_id
lang string
level string
]])

M.category = make_iter("category", "cat", [[
id category_id
title page_title
pages integer
subcats integer
files integer
]])

M.categorylinks = make_iter("category", "cl", [[
from page_id
to page_title
sortkey string
timestamp timestamp
sortkey_prefix string
collation string
type page_type
]])

M.change_tag = make_iter("change_tag", "ct", [[
id integer
rc_id nullable_integer
log_id nullable_integer
rev_id nullable_integer
params nullable_string
tag_id integer
]])

M.change_tag_def = make_iter("change_tag_def", "ctd", [[
id integer
name string
user_defined boolean
count integer
]])

M.externallinks = make_iter("externallinks", "el", [[
id integer
from page_id
to string
index string
index_60 string
]])

M.geo_tags = make_iter("geo_tags", "gt", [[
id integer
page_id page_id
globe string
primary boolean
lat_int integer
lon_int integer
lat nullable_float
lon nullable_float
dim nullable_integer
type nullable_string
name nullable_string
country string
region string
]])

M.image = make_iter("image", "img", [[
name page_title
size integer
width integer
height integer
metadata string
bits integer
media_type string -- actually enum('UNKNOWN','BITMAP','DRAWING','AUDIO','VIDEO','MULTIMEDIA','OFFICE','TEXT','EXECUTABLE','ARCHIVE','3D')
major_mime string -- actually enum('unknown','application','audio','image','text','video','message','model','multipart')
minor_mime string
description_id integer
actor integer
timestamp timestamp
sha1 string
deleted integer
]])

M.imagelinks = make_iter("imagelinks", "il", [[
from page_id
namespace page_namespace
to string
]])

M.iwlinks = make_iter("iwlinks", "iwl", [[
from page_id
prefix string
title page_title
]])

M.langlinks = make_iter("langlinks", "ll", [[
from page_id
lang string
title page_title
]])

M.page = make_iter("page", "page", [[
id page_id
namespace page_namespace
title page_title
restrictions string
is_redirect boolean
is_new boolean
random float
touched timestamp
links_updated nullable_timestamp
latest rev_id
len integer
content_model nullable_string
lang nullable_string
]])

M.page_restrictions = make_iter("page_restrictions", "pr", [[
page page_id
type string
level string
cascade integer
user nullable_integer
expiry nullable_string
id integer
]])

M.pagelinks = make_iter("pagelinks", "pl", [[
from page_id
from_namespace page_namespace
namespace page_namespace
title page_title
]])

M.protected_titles = make_iter("protected_titles", "pt", [[
namespace page_namespace
title page_title
user user_id
reason_id integer
timestamp timestamp
expiry timestamp
create_perm string
]])

M.redirect = make_iter("redirect", "rd", [[
from page_id
namespace page_namespace
title page_title
interwiki nullable_string
fragment nullable_string
]])

M.site_stats = make_iter("site_stats", "ss", [[
row_id integer
total_edits nullable_integer
good_articles nullable_integer
total_pages nullable_integer
users nullable_integer
images nullable_integer
active_users nullable_integer
]])

M.sites = make_iter("sites", "site", [[
id integer
global_key string
type string
group string
source string
language string
protocol string
domain string
data string
forward boolean
config string
]])

M.templatelinks = make_iter("templatelinks", "tl", [[
from page_id
namespace page_namespace
title page_title
from_namespace page_namespace
]])

M.user_former_groups = make_iter("user_former_groups", "ufg", [[
user user_id
group user_group
]])

M.user_groups = make_iter("user_groups", "ug", [[
user user_id
group user_group
expiry timestamp
]])

M.wbc_entity_usage = make_iter("wbc_entity_usage", "eu", [[
row_id integer -- bigint(20), but will probably not overflow 64-bit signed integer soon
entity_id string
aspect string
page_id page_id
]])

return M
