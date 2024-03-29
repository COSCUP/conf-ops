#/bin/bash

db_pwd=$(openssl rand -base64 16 | tr -dc 'a-zA-Z0-9' | head -c 16)
local_db_url="mysql://root:$db_pwd@127.0.0.1/conf-ops"
secret_key=$(openssl rand -base64 32)

docker run --name conf-ops-dev-mysql -e MYSQL_ROOT_PASSWORD=$db_pwd -p 3306:3306 -d mysql:8.0.36

cat <<EOF > ./scripts/env.sh
#/bin/bash

export DATABASE_URL="$local_db_url"
export RUSTFLAGS="-L/opt/homebrew/opt/mysql-client@8.0/lib"
EOF

chmod +x ./scripts/env.sh

source ./scripts/env.sh

config=$(cat "./Rocket.toml.example")
config=${config/<db_url>/$local_db_url}
config=${config/<secret_key>/$secret_key}
echo "$config" > Rocket.toml

mkdir -p public

echo "Please edit Rocket.toml to configure."

