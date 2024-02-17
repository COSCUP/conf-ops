-- Your SQL goes here
CREATE TABLE `projects`(
	`id` VARCHAR(50) NOT NULL,
	`name` VARCHAR(100) NOT NULL,
	`description` TEXT NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	PRIMARY KEY(`id`)
);

CREATE TABLE `users`(
	`id` CHAR(36) NOT NULL,
	`name` VARCHAR(100) NOT NULL,
	`project_id` VARCHAR(50) NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`project_id`) REFERENCES `projects`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `user_emails`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`user_id` CHAR(36) NOT NULL,
	`email` VARCHAR(255) NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`user_id`) REFERENCES `users`(`id`),
	PRIMARY KEY(`id`)
);

CREATE UNIQUE INDEX `email` ON `user_emails` (`email`);

CREATE TABLE `user_sessions`(
	`id` CHAR(36) NOT NULL,
	`user_id` CHAR(36) NOT NULL,
	`user_agent` VARCHAR(255) NOT NULL,
	`ip` VARCHAR(45) NOT NULL,
	`expired_at` TIMESTAMP NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`user_id`) REFERENCES `users`(`id`),
	PRIMARY KEY(`id`)
);

CREATE TABLE `labels`(
	`id` INTEGER NOT NULL AUTO_INCREMENT,
	`project_id` VARCHAR(50) NOT NULL,
	`key` VARCHAR(20) NOT NULL,
	`value` VARCHAR(20) NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`project_id`) REFERENCES `projects`(`id`),
	PRIMARY KEY(`id`)
);

CREATE UNIQUE INDEX `key_value` ON `labels` (`key`, `value`);

CREATE TABLE `users_labels`(
	`user_id` CHAR(36) NOT NULL,
  	`label_id` INTEGER NOT NULL,
	`created_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	`updated_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
	FOREIGN KEY (`user_id`) REFERENCES `users`(`id`),
	FOREIGN KEY (`label_id`) REFERENCES `labels`(`id`),
	PRIMARY KEY(`user_id`, `label_id`)
);

INSERT INTO projects (`id`,`name`,`description`) VALUES(
	'coscup-2024',
	'COSCUP 2024',
	'Conference for Open Source Coders, Users & Promoters'
);

INSERT INTO users (`id`, `name`,`project_id`) VALUES(
	'c3e3e3e3-3e3e-3e3e-3e3e-3e3e3e3e3e3e',
	'yoyoIU',
	'coscup-2024'
);

INSERT INTO user_emails (`user_id`, `email`) VALUES(
	'c3e3e3e3-3e3e-3e3e-3e3e-3e3e3e3e3e3e',
	'yoyo930021+confops@gmail.com'
);

INSERT INTO labels (`project_id`, `key`, `value`) VALUES(
	'coscup-2024',
	'role',
	'admin'
);

SELECT `id` INTO @label_id FROM labels WHERE `key` = 'role' AND `value` = 'admin';

INSERT INTO users_labels (`user_id`, `label_id`) VALUES(
	'c3e3e3e3-3e3e-3e3e-3e3e-3e3e3e3e3e3e',
	@label_id
);
