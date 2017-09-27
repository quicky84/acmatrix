#!/bin/bash

function send-transaction {
    curl -H "Content-Type: application/json" -X POST -d @$1 http://127.0.0.1:8000/api/services/ac/v1/ac/transaction
}

send-transaction ./json/grant-1.json
send-transaction ./json/grant-2.json