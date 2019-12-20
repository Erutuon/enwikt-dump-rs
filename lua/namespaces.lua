local siteinfo = assert(io.open("siteinfo-namespaces.json", "rb")):read "a"
siteinfo = require "cjson".decode(siteinfo)

local namespace_number_to_text = {}

for _, namespace in pairs(siteinfo.query.namespaces) do
  namespace_number_to_text[namespace.id] = namespace["*"]
end

return namespace_number_to_text