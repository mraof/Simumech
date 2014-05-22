package com.mraof.simumech.twitter;

import twitter4j.Status;
import twitter4j.Twitter;
import twitter4j.TwitterFactory;

import com.mraof.simumech.IChat;
import com.mraof.simumech.Main;
import com.mraof.simumech.Util;

public class TwitterChat implements IChat
{
	public void tweet()
	{
		try {
			Twitter twitter = new TwitterFactory().getInstance();
			System.out.println(twitter.getAuthorization().isEnabled());
			String sentence;
		        do
			{
				sentence = Main.markovChain.randomSentence();
			}while(sentence.length() > 140);
			Status status = twitter.updateStatus(sentence);
			System.out.println("Successfully updated the status to [" + status.getText() + "].");
		} catch (Exception e)
		{e.printStackTrace();}
	}

	@Override
	public void message(String message) {
	}

	@Override
	public void command(String message) 
	{
		String splitString[] = Util.splitFirst(message);
		switch(splitString[0].toUpperCase())
		{
		case "TWEET":
			tweet();
			break;
		}
	}

	@Override
	public void quit() {
		// TODO Auto-generated method stub

	}
}
