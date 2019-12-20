local id_to_template = require "template_ids"
local namespace_names = require "namespaces"

local redirects_by_target = setmetatable({}, {
	__index = function(self, k)
		local val = {}
		self[k] = val
		return val
	end,
})
for redirect in require "iter_sql" "redirect" do
	local redirect_title = id_to_template[redirect.from]
	if redirect_title and namespace_names[redirect.namespace] == "Template" then
		table.insert(redirects_by_target[redirect.title], redirect_title)
	end
end

setmetatable(redirects_by_target, nil)

return redirects_by_target