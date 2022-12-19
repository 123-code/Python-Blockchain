    #n determines how much letters we move to encrypt
def generate_key(nums):
    letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    key = {}
    counter = 0
    for i in letters:
        key[i] = letters[(counter+nums)%len(letters)]
        counter +=1

    return key

def encrypt(key,message):
    encrypted = ""
    for i in message:

      if i in key:
        encrypted += key[i]
      else:
        encrypted += i
    return encrypted


key = generate_key(3)
print(key)
message = "JOSE IGNACIO"
cipher = encrypt(key,message)
print(cipher)






    
    





