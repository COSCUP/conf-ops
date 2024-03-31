# conf-ops

We are currently developing and designing a project specifically to handle community workflows for [COSCUP](https://coscup.org). It's in the early stages right now.

## development environment

### frontend
```bash
# install nodejs
curl https://get.volta.sh | bash
volta install node:lts

# install pnpm
corepack enable pnpm

# frontend folder
cd client

# install deps
pnpm install

# start dev
pnpm run dev
```

### backend
```bash
# install docker for database (not requirement)
brew install orbstack

# install rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# install diesel-cli
cargo install diesel-cli

# open database in mysql and generate app config and set envs for dev
source ./scripts/prepare.sh

# set other app config
vi Rocket.toml

# setup database
diesel setup

# run migration
diesel migration run

# revert migration
diesel migration revert

# create migration file
diesel migration generate <filename>

# start dev
cargo watch -i client/ -x run
```

### common
```bash
# start dev in frontend and backend together
./scripts/dev.sh

# remove dev database
./scripts/clean.sh
```

### build docker
```bash
docker build -t conf-ops .
docker run -d \
  --name conf-ops \ # container name
  -v ./app-data/:/usr/src/app/app-data/ \ # app data folder
  -v ./Rocket.toml:/usr/src/app/Rocket.toml \ # app config
  --net host \ # for dev
  --port 8000:8000 \ # for production
  conf-ops
```
