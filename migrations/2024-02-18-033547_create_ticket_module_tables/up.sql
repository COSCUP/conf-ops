-- Your SQL goes here
CREATE TABLE `ticket_schemas`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`title` VARCHAR(100) NOT NULL,
	`description` TEXT NOT NULL,
	`project_id` VARCHAR(50) NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`project_id`) REFERENCES `projects`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_schema_managers`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`ticket_schema_id` INTEGER NOT NULL,
	`target_id` INTEGER NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_schema_id`) REFERENCES `ticket_schemas`(`id`),
	FOREIGN KEY (`target_id`) REFERENCES `targets`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_schema_flows`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`ticket_schema_id` INTEGER NOT NULL,
	`order` INTEGER NOT NULL,
	`operator_id` INTEGER NOT NULL,
	`name` VARCHAR(100) NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_schema_id`) REFERENCES `ticket_schemas`(`id`),
	FOREIGN KEY (`operator_id`) REFERENCES `targets`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_schema_forms`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`ticket_schema_flow_id` INTEGER NOT NULL UNIQUE,
	`expired_at` TIMESTAMP,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_schema_flow_id`) REFERENCES `ticket_schema_flows`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_schema_form_fields`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`ticket_schema_form_id` INTEGER NOT NULL,
	`order` INTEGER NOT NULL,
	`key` VARCHAR(100) NOT NULL,
	`name` VARCHAR(100) NOT NULL,
	`description` TEXT NOT NULL,
	`define` JSON NOT NULL,
	`required` BOOLEAN NOT NULL,
	`editable` BOOLEAN NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_schema_form_id`) REFERENCES `ticket_schema_forms`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_schema_reviews`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`ticket_schema_flow_id` INTEGER NOT NULL,
	`restarted` BOOLEAN NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_schema_flow_id`) REFERENCES `ticket_schema_flows`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `tickets`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`ticket_schema_id` INTEGER NOT NULL,
	`title` VARCHAR(150) NOT NULL,
	`finished` BOOLEAN NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_schema_id`) REFERENCES `ticket_schemas`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_flows`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`ticket_id` INTEGER NOT NULL,
	`user_id` CHAR(36),
	`ticket_schema_flow_id` INTEGER NOT NULL,
	`finished` BOOLEAN NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_id`) REFERENCES `tickets`(`id`),
	FOREIGN KEY (`ticket_schema_flow_id`) REFERENCES `ticket_schema_flows`(`id`),
	FOREIGN KEY (`user_id`) REFERENCES `users`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_form_images`(
	`id` char(64) NOT NULL,
	`ticket_schema_form_field_id` INTEGER NOT NULL,
	`path` VARCHAR(255) NOT NULL,
	`mime` VARCHAR(20) NOT NULL,
	`size` INTEGER UNSIGNED NOT NULL,
	`width` INTEGER UNSIGNED NOT NULL,
	`height` INTEGER UNSIGNED NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_schema_form_field_id`) REFERENCES `ticket_schema_form_fields`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_form_files`(
	`id` char(64) NOT NULL,
	`ticket_schema_form_field_id` INTEGER NOT NULL,
	`path` VARCHAR(255) NOT NULL,
	`mime` VARCHAR(20) NOT NULL,
	`size` INTEGER UNSIGNED NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_schema_form_field_id`) REFERENCES `ticket_schema_form_fields`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_form_answers`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`ticket_flow_id` INTEGER NOT NULL,
	`ticket_schema_form_id` INTEGER NOT NULL,
	`value` JSON NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_flow_id`) REFERENCES `ticket_flows`(`id`),
	FOREIGN KEY (`ticket_schema_form_id`) REFERENCES `ticket_schema_forms`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `ticket_reviews`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`ticket_flow_id` INTEGER NOT NULL,
	`ticket_schema_review_id` INTEGER NOT NULL,
	`approved` BOOLEAN NOT NULL,
	`comment` TEXT,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`ticket_flow_id`) REFERENCES `ticket_flows`(`id`),
	FOREIGN KEY (`ticket_schema_review_id`) REFERENCES `ticket_schema_reviews`(`id`),
	PRIMARY KEY(`id`)
);
