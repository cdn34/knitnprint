terraform {
  backend "s3" {
    bucket       = "knitnprint-staging-terraform-state-739863594156"
    key          = "staging/application/terraform.tfstate"
    region       = "eu-west-1"
    encrypt      = true
    use_lockfile = true
    profile      = "knitnprint-administrator"
  }
}
