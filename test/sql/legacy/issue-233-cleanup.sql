-- can't be the role we want to drop and can't guarantee the name of a role to switch back to
-- so just have another test case to cleanup afterwards
DROP ROLE issue_233;