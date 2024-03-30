-- Your SQL goes here
ALTER TABLE `role_managers`
ADD CONSTRAINT `unique_role_id_target_id` UNIQUE (`role_id`, `target_id`);

ALTER TABLE `targets`
ADD CONSTRAINT `unique_user_id_label_id` UNIQUE (`user_id`, `label_id`);

ALTER TABLE `ticket_flows`
ADD CONSTRAINT `unique_ticket_id_ticket_schema_flow_id` UNIQUE (`ticket_id`, `ticket_schema_flow_id`);

ALTER TABLE `ticket_form_answers`
ADD CONSTRAINT `unique_ticket_flow_id_ticket_schema_form_id` UNIQUE (`ticket_flow_id`, `ticket_schema_form_id`);

ALTER TABLE `ticket_reviews`
ADD CONSTRAINT `unique_ticket_flow_id_ticket_schema_review_id` UNIQUE (`ticket_flow_id`, `ticket_schema_review_id`);

ALTER TABLE `ticket_schema_flows`
ADD CONSTRAINT `unique_ticket_schema_id_order` UNIQUE (`ticket_schema_id`, `order`);

ALTER TABLE `ticket_schema_form_fields`
ADD CONSTRAINT `unique_ticket_schema_form_id_order` UNIQUE (`ticket_schema_form_id`, `order`);

ALTER TABLE `ticket_schema_managers`
ADD CONSTRAINT `unique_ticket_schema_id_target_id` UNIQUE (`ticket_schema_id`, `target_id`);

ALTER TABLE `user_emails`
ADD CONSTRAINT `unique_user_id_email` UNIQUE (`user_id`, `email`);
