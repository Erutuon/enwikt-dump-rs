local utf8proc_map = require 'lutf8proc'.map
local function casefold(str)
	return utf8proc_map(str, 'casefold')
end

local function case_insensitive_comp(a, b)
	local casefolded_a, casefolded_b = casefold(a), casefold(b)
	if casefolded_a ~= casefolded_b then
		return casefolded_a < casefolded_b
	else
		return a < b
	end
end

return { casefold = casefold, comp = case_insensitive_comp }