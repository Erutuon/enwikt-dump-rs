collectgarbage("setpause", 150)

local namespace_names = require "namespaces"

local id_to_template = {}
for page in require "iter_sql" "page" do
	if namespace_names[page.namespace] == "Template" then
		id_to_template[page.id] = page.title
	end
end

return id_to_template