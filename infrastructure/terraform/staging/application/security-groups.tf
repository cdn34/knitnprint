# Public traffic terminates at the load balancer. Application containers are
# never directly reachable from the internet.
resource "aws_security_group" "load_balancer" {
  name        = "${local.name_prefix}-load-balancer"
  description = "Public HTTP and HTTPS entry point"
  vpc_id      = aws_vpc.staging.id

  ingress {
    description = "HTTP redirect to HTTPS"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    description = "Public HTTPS"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    description = "Forward requests to application targets"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${local.name_prefix}-load-balancer"
  }
}

# API, storefront, ClamAV, and operational jobs share one staging task
# boundary. Only the ALB can initiate requests from outside this group.
resource "aws_security_group" "application" {
  name        = "${local.name_prefix}-application"
  description = "KnitNPrint staging application tasks"
  vpc_id      = aws_vpc.staging.id

  ingress {
    description     = "Storefront traffic from the load balancer"
    from_port       = 3000
    to_port         = 3000
    protocol        = "tcp"
    security_groups = [aws_security_group.load_balancer.id]
  }

  ingress {
    description     = "API traffic from the load balancer"
    from_port       = 8080
    to_port         = 8080
    protocol        = "tcp"
    security_groups = [aws_security_group.load_balancer.id]
  }

  ingress {
    description = "Storefront and jobs to the API"
    from_port   = 8080
    to_port     = 8080
    protocol    = "tcp"
    self        = true
  }

  ingress {
    description = "API malware-scanning requests"
    from_port   = 3310
    to_port     = 3310
    protocol    = "tcp"
    self        = true
  }

  egress {
    description = "AWS services, third-party APIs, DNS, and database access"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${local.name_prefix}-application"
  }
}

# PostgreSQL accepts connections only from the application task group and has
# no public address or internet route.
resource "aws_security_group" "database" {
  name        = "${local.name_prefix}-database"
  description = "Private PostgreSQL access"
  vpc_id      = aws_vpc.staging.id

  ingress {
    description     = "PostgreSQL from application tasks"
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [aws_security_group.application.id]
  }

  tags = {
    Name = "${local.name_prefix}-database"
  }
}
