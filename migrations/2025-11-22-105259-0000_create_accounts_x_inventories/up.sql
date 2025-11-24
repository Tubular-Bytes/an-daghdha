CREATE TABLE accounts_x_inventories (
    account_id UUID REFERENCES accounts(id),
    inventory_id UUID REFERENCES inventories(id),
    PRIMARY KEY (account_id, inventory_id)
);