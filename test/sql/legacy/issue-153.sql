create schema test_expand;

create table test_expand.data(pk_data bigint, family_group bigint, first_name text, constraint idx_test_expand_data_pkey primary key (pk_data));

create table test_expand.var(pk_var bigint, pets text, constraint idx_test_expand_var_pkey primary key (pk_var));

insert into test_expand.data(pk_data, family_group, first_name) values(1,1,'mark'); insert into test_expand.data(pk_data, family_group, first_name) values(2,1,'eric'); insert into test_expand.data(pk_data, family_group, first_name) values(3,NULL,'terry');


insert into test_expand.var(pk_var, pets) values(1,'dogs'); insert into test_expand.var(pk_var, pets) values(2,'cats'); insert into test_expand.var(pk_var, pets) values(3,'minions');

CREATE INDEX es_test_expand_var ON test_expand.var USING zombodb ((var.*))
WITH (shards='3', replicas='0');

CREATE INDEX es_test_expand_data ON test_expand.data USING zombodb ((data.*))
WITH (options='pk_data = <test_expand.var.es_test_expand_var>pk_var', shards='3', replicas='0');

CREATE OR REPLACE VIEW test_expand.consolidated_record_view AS  SELECT data.pk_data
                                                                  ,data.family_group
                                                                  ,data.first_name
                                                                  ,var.pets
                                                                  ,data.ctid AS zdb
                                                                FROM test_expand.data
                                                                  LEFT JOIN test_expand.var ON data.pk_data = var.pk_var;

SELECT * FROM test_expand.consolidated_record_view;

SELECT * FROM test_expand.consolidated_record_view where zdb==>'( (#expand<family_group=<this.index>family_group>( ( first_name = "MARK" ) )) )';

SELECT upper(term) term, count FROM zdb.tally('test_expand.es_test_expand_data', 'pets', '0', '^.*', '( (#expand<family_group=<this.index>family_group>( ( first_name = "MARK" ) )) )', 2147483647, 'term');

DROP SCHEMA test_expand CASCADE;