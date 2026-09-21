# Bootstrap state is stored separately from every application stack and from
# the future production environment.
terraform {
  backend "s3" {
    bucket       = "knitnprint-staging-terraform-state-739863594156"
    key          = "staging/bootstrap/terraform.tfstate"
    region       = "eu-west-1"
    encrypt      = true
    use_lockfile = true
    profile      = "knitnprint-administrator"
  }
}
