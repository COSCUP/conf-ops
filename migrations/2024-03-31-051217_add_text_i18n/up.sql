-- Your SQL goes here
ALTER TABLE `projects` RENAME COLUMN `name` TO `name_zh`;
ALTER TABLE `projects` ADD COLUMN `name_en` VARCHAR(100) NOT NULL;

ALTER TABLE `projects` RENAME COLUMN `description` TO `description_zh`;
ALTER TABLE `projects` ADD COLUMN `description_en` TEXT NOT NULL;

ALTER TABLE `roles` RENAME COLUMN `name` TO `name_zh`;
ALTER TABLE `roles` ADD COLUMN `name_en` VARCHAR(50) NOT NULL;

ALTER TABLE `roles` RENAME COLUMN `login_message` TO `login_message_zh`;
ALTER TABLE `roles` ADD COLUMN `login_message_en` TEXT;

ALTER TABLE `roles` RENAME COLUMN `welcome_message` TO `welcome_message_zh`;
ALTER TABLE `roles` ADD COLUMN `welcome_message_en` TEXT;

ALTER TABLE `ticket_schemas` RENAME COLUMN `title` TO `title_zh`;
ALTER TABLE `ticket_schemas` ADD COLUMN `title_en` VARCHAR(100) NOT NULL;

ALTER TABLE `ticket_schemas` RENAME COLUMN `description` TO `description_zh`;
ALTER TABLE `ticket_schemas` ADD COLUMN `description_en` TEXT NOT NULL;

ALTER TABLE `ticket_schema_flows` RENAME COLUMN `name` TO `name_zh`;
ALTER TABLE `ticket_schema_flows` ADD COLUMN `name_en` VARCHAR(100) NOT NULL;

ALTER TABLE `ticket_schema_form_fields` RENAME COLUMN `name` TO `name_zh`;
ALTER TABLE `ticket_schema_form_fields` ADD COLUMN `name_en` VARCHAR(100) NOT NULL;

ALTER TABLE `ticket_schema_form_fields` RENAME COLUMN `description` TO `description_zh`;
ALTER TABLE `ticket_schema_form_fields` ADD COLUMN `description_en` TEXT NOT NULL;
