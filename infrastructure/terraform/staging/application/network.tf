resource "aws_vpc" "staging" {
  cidr_block           = var.vpc_cidr
  enable_dns_support   = true
  enable_dns_hostnames = true

  tags = {
    Name = "${local.name_prefix}-vpc"
  }
}

resource "aws_internet_gateway" "staging" {
  vpc_id = aws_vpc.staging.id

  tags = {
    Name = "${local.name_prefix}-igw"
  }
}

resource "aws_subnet" "staging" {
  for_each = local.subnets

  vpc_id                  = aws_vpc.staging.id
  availability_zone       = each.value.availability_zone
  cidr_block              = cidrsubnet(var.vpc_cidr, 4, each.value.netnum)
  map_public_ip_on_launch = each.value.public

  tags = {
    Name = "${local.name_prefix}-${each.key}"
    Tier = each.value.public ? "public" : "database"
  }
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.staging.id

  tags = {
    Name = "${local.name_prefix}-public"
  }
}

resource "aws_route" "public_ipv4" {
  route_table_id         = aws_route_table.public.id
  destination_cidr_block = "0.0.0.0/0"
  gateway_id             = aws_internet_gateway.staging.id
}

resource "aws_route_table_association" "public" {
  for_each = local.public_subnets

  subnet_id      = aws_subnet.staging[each.key].id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table" "database" {
  for_each = local.database_subnets

  vpc_id = aws_vpc.staging.id

  tags = {
    Name = "${local.name_prefix}-${each.key}"
  }
}

resource "aws_route_table_association" "database" {
  for_each = local.database_subnets

  subnet_id      = aws_subnet.staging[each.key].id
  route_table_id = aws_route_table.database[each.key].id
}
