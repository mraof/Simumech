package com.mraof.simumech.skype;

import com.skype.Friend;
import com.skype.Skype;
import com.skype.SkypeException;

public class SkypeContactAdder implements Runnable
{
	SkypeBot parent;
	public SkypeContactAdder(SkypeBot parent) 
	{
		this.parent = parent;
	}
	@Override
	public void run() 
	{
		while(parent.running)
		{
			try {
				Friend[] waiting;
				waiting = Skype.getContactList().getAllUserWaitingForAuthorization();
				Thread.sleep(10000);
				if(waiting.length != 0)
					for(Friend friend : waiting)
						Skype.getContactList().addFriend(friend, "Accepted");
			} catch (SkypeException e) {e.printStackTrace();} catch (InterruptedException e) {e.printStackTrace();}
		}
		System.out.println("Contact thread finished");
	}

}
