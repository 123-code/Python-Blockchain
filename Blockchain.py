import datetime
import hashlib
# to return block, info in json format 
import json
from flask import Flask, jsonify

print("Python Blockchain")

class PythonBlockchain:
    def __init__(self):
        self.chain  = []
        self.createblock(proof=1,prev_hash='0')

    def createblock(self,proof,prev_hash):
        block = {'index':len(self.chain+1),
        'timestamp':str(datetime.datetime.now())
        ,'prev_hash':prev_hash
        ,'proof':proof
        ,'hash':"0x",
        'data':'Data'}
        self.chain.append(block) 
        return block

    def getpreviousblock(self):
        return self.chain[-1]

    def proof_of_work(self,prev_proof):
        new_proof = 1 
        check_proof = False

        while check_proof is False:
            # sha256 function to implement pow
            hash_operation =  hashlib.sha256(str(new_proof**2 - prev_proof**2 ).encode()).hexdigest()
            if hash_operation[:4]=='0000':
                check_proof = True
            else:
                new_proof += 1
        return new_proof
    
    def hash(self,block):
        encoded_block = json.dumps(block,sort_keys=True).encode()
        return hashlib.sha256(encoded_block).hexdigest()

    def ischainvalid(self,block,chain):
        prev_block = chain[0]
        block_index = 1

        while block_index < len(chain):
            block = chain[block_index]
            if block['prev_hash'] == self.chain[block-1['hash']]:

              return True 




        

        else:
            #ischainvalid(block)
            return False






            
            


    


        
        

