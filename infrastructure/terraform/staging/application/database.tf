resource "aws_db_subnet_group" "staging" {
  name       = "${local.name_prefix}-database"
  subnet_ids = [for name in sort(keys(local.database_subnets)) : aws_subnet.staging[name].id]

  tags = {
    Name = "${local.name_prefix}-database"
  }
}

resource "aws_db_parameter_group" "postgres17" {
  name   = "${local.name_prefix}-postgres17"
  family = "postgres17"

  parameter {
    name         = "rds.force_ssl"
    value        = "1"
    apply_method = "pending-reboot"
  }

  tags = {
    Name = "${local.name_prefix}-postgres17"
  }
}

resource "aws_db_instance" "postgres" {
  identifier = "${local.name_prefix}-postgres"

  engine         = "postgres"
  engine_version = "17.9"
  instance_class = "db.t4g.micro"

  allocated_storage = 20
  storage_type      = "gp3"
  storage_encrypted = true

  db_name                     = "knitnprint"
  username                    = "knitnprint_admin"
  manage_master_user_password = true
  port                        = 5432

  db_subnet_group_name   = aws_db_subnet_group.staging.name
  parameter_group_name   = aws_db_parameter_group.postgres17.name
  vpc_security_group_ids = [aws_security_group.database.id]
  publicly_accessible    = false
  multi_az               = false
  network_type           = "IPV4"

  backup_retention_period = 7
  backup_window           = "03:00-04:00"
  maintenance_window      = "sun:04:00-sun:05:00"
  copy_tags_to_snapshot   = true

  auto_minor_version_upgrade  = true
  allow_major_version_upgrade = false
  apply_immediately           = true

  performance_insights_enabled = false
  monitoring_interval          = 0

  deletion_protection       = true
  skip_final_snapshot       = false
  final_snapshot_identifier = "${local.name_prefix}-postgres-final"
  delete_automated_backups  = true

  tags = {
    Name = "${local.name_prefix}-postgres"
  }
}
