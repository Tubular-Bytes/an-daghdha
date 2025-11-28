insert into accounts ("username", "email", "password_hash", "state")
    values ('brvy', 'no@email.com', '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08', 'active');
    
insert into inventories ("id") values (gen_random_uuid());

insert into accounts_x_inventories (account_id, inventory_id) select a.id as account_id, i.id as inventory_id from accounts as a join inventories as i on 1 = 1 LIMIT 1;

insert into blueprints (slug, name, properties)
    values ('test', 'Test Blueprint', '{"ticks_required": 10}'::jsonb);

insert into inventories_x_buildings (inventory_id, blueprint_slug, progress, status)
    values (
        (select inventory_id from accounts_x_inventories limit 1),
        'test',
        0,
        'in_progress'
    );
