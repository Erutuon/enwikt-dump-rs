local sql_parser = require "parse_sql_dump"
local function iter_sql(table_name)
	local iter = sql_parser[table_name]
	if not iter then
		error("No iter for table " .. table_name)
	end
	local page_sql = assert(io.open(table_name .. ".sql", "rb")):read "a"
	return iter(page_sql)
end

return iter_sql