resource "aws_lb" "application" {
  name               = local.name_prefix
  load_balancer_type = "application"
  internal           = false
  security_groups    = [aws_security_group.load_balancer.id]
  subnets            = [for name in sort(keys(local.public_subnets)) : aws_subnet.staging[name].id]

  enable_deletion_protection = false
  drop_invalid_header_fields = true
  idle_timeout               = 60

  tags = {
    Name = local.name_prefix
  }
}

resource "aws_lb_target_group" "storefront" {
  name        = "${local.name_prefix}-storefront"
  port        = 3000
  protocol    = "HTTP"
  target_type = "ip"
  vpc_id      = aws_vpc.staging.id

  deregistration_delay = 30

  health_check {
    enabled             = true
    path                = "/health"
    matcher             = "200-399"
    interval            = 30
    timeout             = 5
    healthy_threshold   = 2
    unhealthy_threshold = 3
  }

  tags = {
    Name = "${local.name_prefix}-storefront"
  }
}

resource "aws_lb_target_group" "api" {
  name        = "${local.name_prefix}-api"
  port        = 8080
  protocol    = "HTTP"
  target_type = "ip"
  vpc_id      = aws_vpc.staging.id

  deregistration_delay = 30

  health_check {
    enabled             = true
    path                = "/api/health"
    matcher             = "200"
    interval            = 30
    timeout             = 5
    healthy_threshold   = 2
    unhealthy_threshold = 3
  }

  tags = {
    Name = "${local.name_prefix}-api"
  }
}

resource "aws_lb_listener" "http" {
  load_balancer_arn = aws_lb.application.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type = "redirect"

    redirect {
      port        = "443"
      protocol    = "HTTPS"
      status_code = "HTTP_301"
    }
  }
}

resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.application.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = data.aws_acm_certificate.storefront.arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.storefront.arn
  }
}

resource "aws_lb_listener_rule" "api" {
  listener_arn = aws_lb_listener.https.arn
  priority     = 10

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.api.arn
  }

  condition {
    path_pattern {
      values = ["/api", "/api/*"]
    }
  }
}
