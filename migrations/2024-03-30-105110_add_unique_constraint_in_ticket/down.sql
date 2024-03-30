-- This file should undo anything in `up.sql`
ALTER TABLE `role_managers` DROP INDEX `unique_role_id_target_id`
ALTER TABLE `targets` DROP INDEX `unique_user_id_label_id`
ALTER TABLE `ticket_flows` DROP INDEX `unique_ticket_id_ticket_schema_flow_id`
ALTER TABLE `ticket_form_answers` DROP INDEX `unique_ticket_flow_id_ticket_schema_form_id`
ALTER TABLE `ticket_reviews` DROP INDEX `unique_ticket_flow_id_ticket_schema_review_id`
ALTER TABLE `ticket_schema_flows` DROP INDEX `unique_ticket_schema_id_order`
ALTER TABLE `ticket_schema_form_fields` DROP INDEX `unique_ticket_schema_form_id_order`
ALTER TABLE `ticket_schema_managers` DROP INDEX `unique_ticket_schema_id_target_id`
ALTER TABLE `user_emails` DROP INDEX `unique_user_id_email`
