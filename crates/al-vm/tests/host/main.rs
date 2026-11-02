//! The tests of one area in one binary (ADR-037 item 5): fewer binaries to link. Each module is a file of host/.

mod bitmap;
mod business_chart_data;
mod engine_sibling;
mod files;
mod generic_dictionary;
mod gone_instream;
mod gzip;
mod http_add_certificate;
mod http_cert_validation;
mod http_content_get_headers;
mod http_headers;
mod http_request_cookies;
mod last_error_object;
mod media;
mod media_export_unchanged;
mod platform_members;
mod streams;
mod task_scheduler;
mod task_scheduler_after_commit;
mod xml_read_bom;
mod xml_read_stream_options;
