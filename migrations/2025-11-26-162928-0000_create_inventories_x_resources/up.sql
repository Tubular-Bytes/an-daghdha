CREATE TABLE inventories_x_resources (
    inventory_id UUID NOT NULL,
    resource TEXT NOT NULL,
    quantity INT NOT NULL DEFAULT 0,
    PRIMARY KEY (inventory_id, resource),
    FOREIGN KEY (inventory_id) REFERENCES inventories(id) ON DELETE CASCADE,
    FOREIGN KEY (resource) REFERENCES resources(slug) ON DELETE CASCADE
);