#/bin/bash

db_pwd=$(openssl rand -base64 16 | tr -dc 'a-zA-Z0-9' | head -c 16)

docker run --name conf-ops-dev-mysql -e MYSQL_ROOT_PASSWORD=$db_pwd -p 3306:3306 -d mysql:8.0.36

cat <<EOF > ./scripts/env.sh
#/bin/bash

export DATABASE_URL="mysql://root:$db_pwd@localhost/conf-ops"
export RUSTFLAGS="-L/opt/homebrew/opt/mysql-client@8.0/lib"
EOF

chmod +x ./scripts/env.sh

source ./scripts/env.sh

cp ./Rocket.toml.example ./Rocket.toml

cat <<EOF >> ./Rocket.toml

[default.databases.main_db]
url = "mysql://root:$db_pwd@localhost/conf-ops"
EOF

mkdir -p public

echo "Please edit Rocket.toml to configure."

